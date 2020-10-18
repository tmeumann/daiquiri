use std::sync::mpsc::Sender;
use libpowerdna_sys::DQ_ACB_DATA_TSCOPY;
use crate::bootstrap::ChannelConfig;
use crate::bootstrap::BoardConfig;
use std::mem::size_of;
use std::thread::spawn;
use libpowerdna_sys::DQ_AI201_GAIN_1_100;
use libpowerdna_sys::DQ_AI201_GAIN_2_100;
use libpowerdna_sys::DQ_AI201_GAIN_5_100;
use libpowerdna_sys::DQ_AI201_GAIN_10_100;
use crate::powerdna::results::PowerDnaError;
use std::sync::Arc;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::atomic::AtomicBool;
use libpowerdna_sys::DqConvRaw2ScalePdc;
use libpowerdna_sys::DqAcbGetScansCopy;
use libpowerdna_sys::DqeWaitForEvent;
use libpowerdna_sys::DqeEnable;
use libpowerdna_sys::pDATACONV;
use libpowerdna_sys::DqConvGetDataConv;
use libpowerdna_sys::DqConvFillConvData;
use libpowerdna_sys::DQ_eBufferDone;
use libpowerdna_sys::DQ_ePacketOOB;
use libpowerdna_sys::DQ_eBufferError;
use libpowerdna_sys::DQ_ePacketLost;
use libpowerdna_sys::DQ_eFrameDone;
use libpowerdna_sys::DqeSetEvent;
use libpowerdna_sys::DQ_AI201_MODEFIFO;
use libpowerdna_sys::DQ_LN_STREAMING;
use libpowerdna_sys::DQ_LN_CLCKSRC0;
use libpowerdna_sys::DQ_LN_IRQEN;
use libpowerdna_sys::DQ_LN_GETRAW;
use libpowerdna_sys::DQ_LN_ACTIVE;
use libpowerdna_sys::DQ_LN_ENABLED;
use libpowerdna_sys::DqAcbInitOps;
use libpowerdna_sys::DQ_LNCL_TIMESTAMP;
use libpowerdna_sys::DQ_ACB_DATA_RAW;
use libpowerdna_sys::DQ_ACB_DIRECTION_INPUT;
use libpowerdna_sys::DQ_ACBMODE_CYCLE;
use libpowerdna_sys::DQACBCFG;
use libpowerdna_sys::DqAcbDestroy;
use libpowerdna_sys::pDQBCB;
use libpowerdna_sys::DQ_SS0IN;
use libpowerdna_sys::DqAcbCreate;
use libpowerdna_sys::STS_FW_OPER_MODE;
use libpowerdna_sys::STS_FW;
use libpowerdna_sys::DQ_MAXDEVN;
use libpowerdna_sys::DQ_LASTDEV;
use libpowerdna_sys::DqCmdReadStatus;
use libpowerdna_sys::DQ_IOMODE_CFG;
use libpowerdna_sys::DqCmdSetMode;
use libpowerdna_sys::DqCloseIOM;
use libpowerdna_sys::DQ_UDP_DAQ_PORT;
use libpowerdna_sys::DqOpenIOM;
use libpowerdna_sys::DqStopDQEngine;
use libpowerdna_sys::pDQE;
use std::ptr::null_mut;
use libpowerdna_sys::DqStartDQEngine;
use std::ffi::CString;
use libpowerdna_sys::DqCleanUpDAQLib;
use libpowerdna_sys::DqInitDAQLib;
use std::convert::TryInto;
use serde::{Deserialize};

use thiserror::Error;

#[macro_use]
pub mod results;

const TIMEOUT: u32 = 200;
const EVENT_TIMEOUT: i32 = 1000;
const CFG201: u32 = DQ_LN_ENABLED | DQ_LN_ACTIVE | DQ_LN_GETRAW | DQ_LN_IRQEN | DQ_LN_CLCKSRC0 | DQ_LN_STREAMING | DQ_AI201_MODEFIFO;

#[derive(Deserialize, Debug)]
#[repr(u32)]
pub enum Gain {
    One = DQ_AI201_GAIN_1_100,
    Two = DQ_AI201_GAIN_2_100,
    Five = DQ_AI201_GAIN_5_100,
    Ten = DQ_AI201_GAIN_10_100,
}

#[derive(Error, Debug)]
pub enum DaqError {
    #[error("Failed to allocate inbound data buffer.")]
    BufferError,
    #[error("Internal error.")]
    PowerDnaError {
        #[from]
        source: PowerDnaError,
    },
}


pub struct Daq {
    handle: i32,
    dqe: Arc<DqEngine>,
}

unsafe impl Send for Daq {}

impl Daq {
    pub fn new(dqe: Arc<DqEngine>, ip: String) -> Result<Self, PowerDnaError> {
        // TODO introduce phantom data to track dqe lifetime?
        let mut handle = 0;
        let config = null_mut();
        
        let ip_ptr = CString::new(ip.as_str()).expect("Failed to allocate memory for IP address.").into_raw();
        let result = parse_err!(DqOpenIOM(ip_ptr, DQ_UDP_DAQ_PORT as u16, TIMEOUT, &mut handle, config));
        unsafe {
            let _ = CString::from_raw(ip_ptr);  // reclaims memory
        }

        match result {
            Err(err) => Err(err),
            Ok(_) => Ok(Daq{
                handle,
                dqe,
            })
        }
    }

    fn create_acb(&self, device: u8) -> Result<pDQBCB, PowerDnaError> {
        let mut bcb: pDQBCB = null_mut();
        // mutation
        parse_err!(DqAcbCreate(self.dqe.dqe, self.handle, device as u32, DQ_SS0IN, &mut bcb))?;
        Ok(bcb)
    }

    fn enter_config_mode(&self, device: u8) -> Result<(), PowerDnaError> {
        let devices: u8 = device | (DQ_LASTDEV as u8);
        let mut num_devices: u32 = 1;
        let mut status_buffer: [u32; DQ_MAXDEVN as usize + 1] = [0; DQ_MAXDEVN as usize + 1];
        let mut status_size: u32 = status_buffer.len() as u32;

        // mutation
        parse_err!(DqCmdReadStatus(self.handle, &devices, &mut num_devices, &mut status_buffer[0], &mut status_size))?;
        
        if status_buffer[STS_FW as usize] & STS_FW_OPER_MODE != 0 {
            parse_err!(DqCmdSetMode(self.handle, DQ_IOMODE_CFG, 1 << (device as u32 & DQ_MAXDEVN)))?;
        }

        Ok(())
    }

    fn get_data_converter(&self, device: u8, channels: &Vec<u32>) -> Result<pDATACONV, PowerDnaError> {
        parse_err!(DqConvFillConvData(self.handle, device as i32, DQ_SS0IN as i32, channels.as_ptr(), channels.len() as u32))?;
        let mut pdc: pDATACONV = null_mut();
        // mutation
        parse_err!(DqConvGetDataConv(self.handle, device as i32, &mut pdc))?;
        Ok(pdc)
    }
}

impl Drop for Daq {
    fn drop(&mut self) {
        match parse_err!(DqCloseIOM(self.handle)) {
            Err(err) => {
                eprintln!("DqCloseIOM failed. Error: {:?}", err);
            },
            Ok(_) => {},
        };
    }
}

fn sampler(board: Arc<Ai201>, stop: Arc<AtomicBool>, mut raw_buffer: Vec<u16>, buffer_size: usize, tx: Sender<(String, Vec<u8>)>, topic: String) {
    while !stop.load(SeqCst) {
        let mut events: u32 = 0;
        
        while events & DQ_eFrameDone == 0 {
            match parse_err!(DqeWaitForEvent(&board.bcb, 1, 0, EVENT_TIMEOUT, &mut events)) {
                Err(PowerDnaError::TimeoutError) => {
                    continue;
                },
                Err(err) => {
                    eprintln!("DqeWaitForEvent failed. Error: {:?}", err);
                    break;
                },
                Ok(_) => {},
            };

            if events & (DQ_ePacketLost|DQ_eBufferError|DQ_ePacketOOB) != 0 {  // TODO recover from errors
                if events & DQ_ePacketLost != 0 {
                    eprintln!("AI:DQ_ePacketLost");
                }
                if events & DQ_eBufferError != 0 {
                    eprintln!("AI:DQ_eBufferError");
                }
                if events & DQ_ePacketOOB != 0 {
                    eprintln!("AI:DQ_ePacketOOB");
                }
                break;
            }
        }

        let framesize: u32 = board.acb_cfg.framesize;
        let mut received_scans: u32 = 0;
        let mut remaining_scans: u32 = 0;

        let buffer_ptr = raw_buffer.as_mut_ptr() as *mut i8;
        
        match parse_err!(DqAcbGetScansCopy(board.bcb, buffer_ptr, framesize, framesize, &mut received_scans, &mut remaining_scans)) {
            Err(err) => {
                eprintln!("DqAcbGetScansCopy failed. Error: {:?}", err);
                break;
            },
            Ok(_) => {},
        };

        let chans = board.channels.len() as u32;
        let mut scaled_buffer: Vec<u8> = vec![0; buffer_size * size_of::<f64>()];

        match parse_err!(DqConvRaw2ScalePdc(board.pdc, board.channels.as_ptr(), chans, received_scans * chans, buffer_ptr, scaled_buffer.as_mut_ptr() as *mut f64)) {
            Err(err) => {
                eprintln!("DqConvRaw2ScalePdc failed. Error: {:?}", err);
                break;
            },
            Ok(_) => {},
        };

        match tx.send((topic.clone(), scaled_buffer)) {
            Ok(_) => {},
            Err(_) => break,  // TODO log me
        };
    }
}


pub struct SignalStream {
    board: Arc<Ai201>,
    stop: Arc<AtomicBool>,
}

impl SignalStream {
    pub fn new(daq: Daq, freq: u32, board_config: &BoardConfig, tx: Sender<(String, Vec<u8>)>, topic: String) -> Result<SignalStream, DaqError> {
        // let Ai201 { mut bcb, acb_cfg, .. } = board;

        let board = Ai201::new(daq, freq, board_config)?;

        let buffer_size = board.buffer_size()?;
        let raw_buffer = vec![0; buffer_size];

        let board = Arc::new(board);
        let cloned_board = Arc::clone(&board);
        let stop = Arc::new(AtomicBool::new(false));
        let cloned_stop = stop.clone();

        spawn(move || {
            sampler(cloned_board, cloned_stop, raw_buffer, buffer_size, tx, topic)
        });

        Ok(SignalStream {
            board,
            stop,
        })
    }

    pub fn start(&self) -> Result<(), DaqError> {
        parse_err!(DqeEnable(1, &self.board.bcb, 1, 0))?;
        Ok(())
    }

    pub fn stop(&self) -> Result<(), DaqError> {
        parse_err!(DqeEnable(0, &self.board.bcb, 1, 0))?;
        Ok(())
    }
}

impl Drop for SignalStream {
    fn drop(&mut self) {
        match self.stop() {
            Err(err) => {
                eprintln!("DqeEnable -> false failed. Error: {:?}", err);
            },
            Ok(_) => {},
        };
        self.stop.store(true, SeqCst);
    }
}



pub struct Ai201 {
    bcb: pDQBCB,
    channels: Vec<u32>,
    pdc: pDATACONV,
    acb_cfg: DQACBCFG,
    daq: Daq,  // TODO change to phantom data
}

impl Ai201 {
    fn new(daq: Daq, freq: u32, board_config: &BoardConfig) -> Result<Self, PowerDnaError> {
        let BoardConfig { device, channels } = board_config;
        daq.enter_config_mode(*device)?;
        let bcb = daq.create_acb(*device)?;

        let mut channel_list: Vec<u32> = channels.iter().map(|ChannelConfig { id, gain }| {
            let gain_mask = match gain {
                &10 => DQ_AI201_GAIN_10_100,
                &5  => DQ_AI201_GAIN_5_100,
                &2  => DQ_AI201_GAIN_2_100,
                _  => DQ_AI201_GAIN_1_100, // TODO handle invalid values
            };
            *id as u32 | (gain_mask << 8)
        }).collect();
        // TODO sort out these weird timestamp channels
        // channel_list.push(channels.len() as u32);
        // channel_list.push(DQ_LNCL_TIMESTAMP);

        let mut acb_cfg = DQACBCFG::empty();

        acb_cfg.samplesz = size_of::<u16>() as u32;  // size of single reading
        acb_cfg.scansz = channel_list.len() as u32;  // number of readings (timestamp is equivalent to 2 readings)
        acb_cfg.framesize = 1000;  // frame size TODO
        acb_cfg.frames = 4;  // # of frames TODO
        acb_cfg.mode = DQ_ACBMODE_CYCLE;
        acb_cfg.dirflags = DQ_ACB_DIRECTION_INPUT | DQ_ACB_DATA_RAW; // | DQ_ACB_DATA_TSCOPY;

        let mut card_cfg = CFG201;
        let mut actual_freq = freq as f32;
        let mut num_channels = channel_list.len() as u32;

        // mutation
        parse_err!(DqAcbInitOps(bcb, &mut card_cfg, null_mut(), null_mut(), &mut actual_freq, null_mut(), &mut num_channels, channel_list.as_mut_ptr(), null_mut(), &mut acb_cfg))?;
        parse_err!(DqeSetEvent(bcb, DQ_eFrameDone | DQ_ePacketLost | DQ_eBufferError | DQ_ePacketOOB | DQ_eBufferDone))?;
        
        let pdc = daq.get_data_converter(*device, &channel_list)?;

        Ok(Ai201 {
            bcb,
            channels: channel_list,
            pdc,
            acb_cfg,
            daq,
        })
    }

    fn buffer_size(&self) -> Result<usize, DaqError> {
        match (self.acb_cfg.framesize * self.acb_cfg.scansz).try_into() {
            Err(_) => Err(DaqError::BufferError),
            Ok(val) => Ok(val),
        }
    }
}

unsafe impl Send for Ai201 {}
unsafe impl Sync for Ai201 {}

impl Drop for Ai201 {
    fn drop(&mut self) {
        match parse_err!(DqAcbDestroy(self.bcb)) {
            Err(err) => {
                eprintln!("DqAcbDestroy failed. Error: {}", err);
            },
            Ok(_) => {},
        };
    }
}



pub struct DqEngine {
    dqe: pDQE,
}

unsafe impl Send for DqEngine {}

impl DqEngine {
    pub fn new(clock_period: u32) -> Result<Self, PowerDnaError> {
        let mut dqe = null_mut();
        
        unsafe {
            DqInitDAQLib();
        }
        parse_err!(DqStartDQEngine(clock_period, &mut dqe, std::ptr::null_mut()))?;

        Ok(
            DqEngine {
                dqe,
            }
        )
    }
}

impl Drop for DqEngine {
    fn drop(&mut self) {
        match parse_err!(DqStopDQEngine(self.dqe)) {
            Err(err) => {
                eprintln!("DqStopDQEngine failed. Error: {:?}", err);
            },
            Ok(_) => {},
        };
        unsafe {
            DqCleanUpDAQLib();
        }
    }
}



trait Empty {
    fn empty() -> Self;
}

impl Empty for DQACBCFG {
    fn empty() -> Self {
        Self {
            dirflags: 0,
            eucoeff: 0.0,
            euconvert: None,
            euoffset: 0.0,
            frames: 0,
            framesize: 0,
            hostringsz: 0,
            hwbufsize: 0,
            maxpktsize: 0,
            mode: 0,
            ppevent: 0,
            samplesz: 0,
            scansz: 0,
            valuesz: 0,
            wtrmark: 0,
        }
    }
}
