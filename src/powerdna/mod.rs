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
use std::collections::HashMap;
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

use thiserror::Error;

#[macro_use]
pub mod results;

const TIMEOUT: u32 = 200;
const EVENT_TIMEOUT: i32 = 1000;
const CFG201: u32 = DQ_LN_ENABLED | DQ_LN_ACTIVE | DQ_LN_GETRAW | DQ_LN_IRQEN | DQ_LN_CLCKSRC0 | DQ_LN_STREAMING | DQ_AI201_MODEFIFO;

#[derive(Error, Debug)]
pub enum DaqError {
    #[error("Failed to allocate inbound data buffer.")]
    BufferError,
    #[error("Internal error.")]
    PowerDnaError {
        #[from]
        source: PowerDnaError,
    },
    #[error("DAQ already in use.")]
    DaqInUseError,
    #[error("Unknown error.")]
    UnknownError,
}


pub struct Daq {
    pub handle: i32,
    dqe: pDQE,
    stream: Option<SignalStream>,
}

impl Daq {
    fn new(ip: &String, dqe: pDQE) -> Result<Self, PowerDnaError> {
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
                stream: None,
            })
        }
    }

    pub fn stream(&mut self, devices: Vec<u8>, freq: u32, stop: Arc<AtomicBool>) -> Result<&mut Option<SignalStream>, DaqError> {
        self.stream = None;
        let boards = devices.into_iter().map(|dev_n| { Ai201::new(self.dqe, self.handle, dev_n, freq) }).collect::<Result<Vec<Ai201>, PowerDnaError>>()?;
        self.stream = Some(SignalStream::new(boards, stop)?);
        Ok(&mut self.stream)
    }
}

impl Drop for Daq {
    fn drop(&mut self) {
        self.stream = None;
        match parse_err!(DqCloseIOM(self.handle)) {
            Err(err) => {
                eprintln!("DqCloseIOM failed. Error: {:?}", err);
            },
            Ok(_) => {},
        };
    }
}



// TODO keep reference to boards, not config values
pub struct SignalStream {
    boards: Vec<Ai201>,
    buffer_size: usize,
    raw_buffer: Vec<u16>,
    stop: Arc<AtomicBool>,
}

impl SignalStream {
    fn new(boards: Vec<Ai201>, stop: Arc<AtomicBool>) -> Result<SignalStream, DaqError> {
        let Ai201 { mut bcb, acb_cfg, .. } = boards[0];
        let buffer_size: usize;

        match (acb_cfg.framesize * acb_cfg.frames * acb_cfg.scansz).try_into() {
            Err(_) => {
                return Err(DaqError::BufferError);
            },
            Ok(val) => {
                buffer_size = val;
            }
        };
        let raw_buffer = vec![0; buffer_size];

        parse_err!(DqeEnable(1, &mut bcb, 1, 0))?;

        Ok(SignalStream {
            boards,
            buffer_size,
            raw_buffer,
            stop,
        })
    }
}

impl Iterator for SignalStream {
    type Item = Vec<f64>;  // TODO return error values?

    fn next(&mut self) -> std::option::Option<<Self as std::iter::Iterator>::Item> {
        if self.stop.load(SeqCst) {
            return None;
        };

        let Ai201 { bcb, acb_cfg, channels, pdc, .. } = &mut self.boards[0];

        let mut events: u32 = 0;
        
        while events & DQ_eFrameDone == 0 {
            match parse_err!(DqeWaitForEvent(bcb, 1, 0, EVENT_TIMEOUT, &mut events)) {
                Err(PowerDnaError::TimeoutError) => {
                    continue;
                },
                Err(err) => {
                    eprintln!("DqeWaitForEvent failed. Error: {:?}", err);
                    return None;
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
                return None;
            }
        }

        let framesize: u32 = acb_cfg.framesize;
        let mut received_scans: u32 = 0;
        let mut remaining_scans: u32 = 0;

        let buffer_ptr = self.raw_buffer.as_mut_ptr() as *mut i8;
        
        match parse_err!(DqAcbGetScansCopy(*bcb, buffer_ptr, framesize, framesize, &mut received_scans, &mut remaining_scans)) {
            Err(err) => {
                eprintln!("DqAcbGetScansCopy failed. Error: {:?}", err);
                return None;
            },
            Ok(_) => {},
        };

        let chans = channels.len() as u32;
        let mut scaled_buffer = vec![0.0; self.buffer_size];

        match parse_err!(DqConvRaw2ScalePdc(*pdc, channels.as_mut_ptr(), chans, received_scans * chans, buffer_ptr, scaled_buffer.as_mut_ptr())) {
            Err(err) => {
                eprintln!("DqConvRaw2ScalePdc failed. Error: {:?}", err);
                return None;
            },
            Ok(_) => {},
        };

        Some(scaled_buffer)
    }
}

impl Drop for SignalStream {
    fn drop(&mut self) {
        match parse_err!(DqeEnable(0, &mut self.boards[0].bcb, 1, 0)) {
            Err(err) => {
                eprintln!("DqeEnable -> false failed. Error: {:?}", err);
            },
            Ok(_) => {},
        };
        self.boards.clear();
    }
}



pub struct Ai201 {
    bcb: pDQBCB,
    channels: Vec<u32>,
    pdc: pDATACONV,
    acb_cfg: DQACBCFG,
}

impl Ai201 {
    fn new(dqe: pDQE, handle: i32, dev_n: u8, freq: u32) -> Result<Self, PowerDnaError> {
        // TODO consider powering down between sampling sessions?
        let mut device: u8 = dev_n | (DQ_LASTDEV as u8);
        let mut num_devices: u32 = 1;
        let mut status_buffer: [u32; DQ_MAXDEVN as usize + 1] = [0; DQ_MAXDEVN as usize + 1];
        let mut status_size: u32 = status_buffer.len() as u32;

        parse_err!(DqCmdReadStatus(handle, &mut device, &mut num_devices, &mut status_buffer[0], &mut status_size))?;
        
        if status_buffer[STS_FW as usize] & STS_FW_OPER_MODE != 0 {
            parse_err!(DqCmdSetMode(handle, DQ_IOMODE_CFG, 1 << (device as u32 & DQ_MAXDEVN)))?;
        }

        let mut bcb: pDQBCB = null_mut();

        parse_err!(DqAcbCreate(dqe, handle, dev_n as u32, DQ_SS0IN, &mut bcb))?;

        let mut channels: Vec<u32> = (0..26).collect();
        channels[24] = 24;
        channels[25] = DQ_LNCL_TIMESTAMP;

        let mut acb_cfg = DQACBCFG::empty();

        acb_cfg.samplesz = 16;  // size of single reading
        acb_cfg.scansz = 26;  // number of readings (incl. timestamp, which is equivalent to 2 readings)
        acb_cfg.framesize = 1000;  // frame size TODO
        acb_cfg.frames = 4;  // # of frames TODO
        acb_cfg.mode = DQ_ACBMODE_CYCLE;
        acb_cfg.samplesz = 16;
        acb_cfg.dirflags = DQ_ACB_DIRECTION_INPUT | DQ_ACB_DATA_RAW | DQ_ACB_DATA_RAW;

        let mut card_cfg = CFG201;
        let mut actual_freq = freq as f32;
        let mut num_channels = 26;

        parse_err!(DqAcbInitOps(bcb, &mut card_cfg, null_mut(), null_mut(), &mut actual_freq, null_mut(), &mut num_channels, channels.as_mut_ptr(), null_mut(), &mut acb_cfg))?;
        parse_err!(DqeSetEvent(bcb, DQ_eFrameDone | DQ_ePacketLost | DQ_eBufferError | DQ_ePacketOOB | DQ_eBufferDone))?;
        
        parse_err!(DqConvFillConvData(handle, dev_n as i32, DQ_SS0IN as i32, channels.as_mut_ptr(), num_channels))?;
        let mut pdc: pDATACONV = null_mut();
        parse_err!(DqConvGetDataConv(handle, dev_n as i32, &mut pdc))?;

        Ok(Ai201 {
            bcb,
            channels,
            pdc,
            acb_cfg,
        })
    }
}

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
    daqs: HashMap<String, Daq>,
}

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
                daqs: HashMap::new(),
            }
        )
    }

    pub fn open_daq(&mut self, ip: String) -> Result<&mut Daq, DaqError> {
        if self.daqs.contains_key(&ip) {
            Err(DaqError::DaqInUseError)
        } else {
            let daq = Daq::new(&ip, self.dqe)?;
            self.daqs.insert(ip.clone(), daq);
            match self.daqs.get_mut(&ip) {
                Some(daq) => Ok(daq),
                None => Err(DaqError::UnknownError),
            }
        }
    }
}

impl Drop for DqEngine {
    fn drop(&mut self) {
        self.daqs.clear();
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
