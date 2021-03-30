use crate::config::{BoardConfig, ChannelConfig, OutputConfig};
use crate::daq::Daq;
use crate::engine::InterfaceType;
use crate::results::PowerDnaError;
use powerdna_sys::{
    pDATACONV, pDQBCB, DQ_eBufferDone, DQ_eBufferError, DQ_eFrameDone, DQ_ePacketLost,
    DQ_ePacketOOB, DqAcbGetScansCopy, DqAcbInitOps, DqAcbPutScansCopy, DqConvRaw2ScalePdc,
    DqeSetEvent, DqeWaitForEvent, DQACBCFG, DQ_ACBMODE_CYCLE, DQ_ACBMODE_RECYCLED, DQ_ACB_DATA_RAW,
    DQ_ACB_DIRECTION_INPUT, DQ_ACB_DIRECTION_OUTPUT, DQ_AI201_GAIN_10_100, DQ_AI201_GAIN_1_100,
    DQ_AI201_GAIN_2_100, DQ_AI201_GAIN_5_100, DQ_AI201_MODEFIFO, DQ_LN_ACTIVE, DQ_LN_CLCKSRC0,
    DQ_LN_CVCKSRC0, DQ_LN_ENABLED, DQ_LN_GETRAW, DQ_LN_IRQEN, DQ_LN_STREAMING,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::{mem, ptr};

const CFG201: u32 = DQ_LN_ENABLED
    | DQ_LN_ACTIVE
    | DQ_LN_GETRAW
    | DQ_LN_IRQEN
    | DQ_LN_CLCKSRC0
    | DQ_LN_STREAMING
    | DQ_AI201_MODEFIFO;
const CFG405: u32 = DQ_LN_ENABLED | DQ_LN_ACTIVE | DQ_LN_IRQEN | DQ_LN_CVCKSRC0 | DQ_LN_STREAMING;
const EVENT_TIMEOUT: i32 = 1000;

pub(crate) trait Bcb {
    fn bcb(&self) -> pDQBCB;
}

pub struct Ai201 {
    bcb: pDQBCB,
    channels: Vec<u32>,
    pdc: pDATACONV,
    acb_cfg: DQACBCFG,
    daq: Arc<Daq>,
    buffer_size: usize,
    out: Sender<Vec<f64>>,
}

impl Ai201 {
    pub(crate) fn new(
        daq: Arc<Daq>,
        freq: u32,
        frame_size: u32,
        board_config: &BoardConfig,
        out: Sender<Vec<f64>>,
    ) -> Result<Self, PowerDnaError> {
        let BoardConfig { device, channels } = board_config;
        daq.enter_config_mode(*device)?;
        let bcb = daq.create_acb(*device, InterfaceType::Input)?;

        let mut channel_list: Vec<u32> = channels
            .iter()
            .map(|ChannelConfig { id, gain }| {
                let gain_mask = match gain {
                    &10 => DQ_AI201_GAIN_10_100,
                    &5 => DQ_AI201_GAIN_5_100,
                    &2 => DQ_AI201_GAIN_2_100,
                    _ => DQ_AI201_GAIN_1_100, // TODO handle invalid values
                };
                *id as u32 | (gain_mask << 8)
            })
            .collect();
        // TODO sort out these weird timestamp channels
        // channel_list.push(channels.len() as u32);
        // channel_list.push(DQ_LNCL_TIMESTAMP);

        let mut acb_cfg = DQACBCFG::empty();

        acb_cfg.samplesz = mem::size_of::<u16>() as u32; // size of single reading
        acb_cfg.scansz = channel_list.len() as u32; // number of readings (timestamp is equivalent to 2 readings)
        acb_cfg.framesize = frame_size;
        acb_cfg.frames = 12; // # of frames in circular buffer TODO
        acb_cfg.mode = DQ_ACBMODE_CYCLE;
        acb_cfg.dirflags = DQ_ACB_DIRECTION_INPUT | DQ_ACB_DATA_RAW; // | DQ_ACB_DATA_TSCOPY;

        let mut card_cfg = CFG201;
        let mut actual_freq = freq as f32;
        let mut num_channels = channel_list.len() as u32;

        // mutation
        parse_err!(DqAcbInitOps(
            bcb,
            &mut card_cfg,
            ptr::null_mut(),
            ptr::null_mut(),
            &mut actual_freq,
            ptr::null_mut(),
            &mut num_channels,
            channel_list.as_mut_ptr(),
            ptr::null_mut(),
            &mut acb_cfg
        ))?;
        parse_err!(DqeSetEvent(
            bcb,
            DQ_eFrameDone | DQ_ePacketLost | DQ_eBufferError | DQ_ePacketOOB | DQ_eBufferDone
        ))?;

        let pdc = daq.get_data_converter(*device, &channel_list)?;

        // TODO does this need to be bigger? ie. * acb_cfg.frames
        let buffer_size = (acb_cfg.framesize * acb_cfg.scansz) as usize;

        Ok(Ai201 {
            bcb,
            channels: channel_list,
            pdc,
            acb_cfg,
            daq,
            buffer_size,
            out,
        })
    }

    pub(crate) fn sample(&self, stop: Arc<AtomicBool>) {
        let mut raw_buffer = vec![0; self.buffer_size];
        'outer: loop {
            let events = match self.wait_for_event() {
                Err(PowerDnaError::TimeoutError) => {
                    match stop.load(Ordering::SeqCst) {
                        true => break,
                        false => continue,
                    };
                }
                Err(err) => {
                    eprintln!("DqeWaitForEvent failed. Error: {:?}", err);
                    break;
                }
                Ok(val) => val,
            };

            if events & DQ_ePacketLost != 0 {
                eprintln!("AI:DQ_ePacketLost");
            }
            if events & DQ_eBufferError != 0 {
                eprintln!("AI:DQ_eBufferError");
            }
            if events & DQ_ePacketOOB != 0 {
                eprintln!("AI:DQ_ePacketOOB");
            }

            if events & DQ_eFrameDone == 0 {
                continue;
            }

            let scaled_data = match self.get_scaled_data(&mut raw_buffer) {
                Ok(val) => val,
                Err(_) => {
                    eprintln!("Failed to get scaled data. Skipping frame!");
                    continue;
                }
            };

            for frame in scaled_data {
                match self.out.send(frame) {
                    Ok(_) => {}
                    Err(_) => break 'outer, // TODO log me
                };
            }
        }
    }

    fn wait_for_event(&self) -> Result<u32, PowerDnaError> {
        let mut events: u32 = 0;
        parse_err!(DqeWaitForEvent(&self.bcb, 1, 0, EVENT_TIMEOUT, &mut events))?;
        Ok(events)
    }

    fn get_scaled_data(&self, raw_buffer: &mut Vec<u16>) -> Result<Vec<Vec<f64>>, PowerDnaError> {
        let framesize: u32 = self.acb_cfg.framesize;
        let mut received_scans: u32 = 0;
        let mut remaining_scans: u32 = 0;

        let buffer_ptr = raw_buffer.as_mut_ptr() as *mut i8;

        let mut scaled_frames: Vec<Vec<f64>> = vec![];

        let mut data_available = true;

        while data_available {
            parse_err!(DqAcbGetScansCopy(
                self.bcb,
                buffer_ptr,
                framesize,
                framesize,
                &mut received_scans,
                &mut remaining_scans
            ))?;

            let chans = self.channels.len() as u32;
            let mut scaled_buffer: Vec<f64> = vec![0.0; self.buffer_size];

            parse_err!(DqConvRaw2ScalePdc(
                self.pdc,
                self.channels.as_ptr(),
                chans,
                received_scans * chans,
                buffer_ptr,
                scaled_buffer.as_mut_ptr() as *mut f64
            ))?;

            scaled_frames.push(scaled_buffer);

            data_available = remaining_scans > framesize;
        }

        Ok(scaled_frames)
    }
}

impl Bcb for Ai201 {
    fn bcb(&self) -> pDQBCB {
        self.bcb
    }
}

unsafe impl Send for Ai201 {}
unsafe impl Sync for Ai201 {}

impl Drop for Ai201 {
    fn drop(&mut self) {
        match self.daq.destroy_acb(self.bcb) {
            Err(err) => {
                eprintln!("DqAcbDestroy failed. Error: {}", err);
            }
            Ok(_) => {}
        };
    }
}

pub struct Dio405 {
    bcb: pDQBCB,
    daq: Arc<Daq>,
    output_buffer: Vec<u32>,
}

impl Dio405 {
    pub(crate) fn new(
        daq: Arc<Daq>,
        freq: u32,
        frame_size: u32,
        board_config: &OutputConfig,
    ) -> Result<Self, PowerDnaError> {
        let OutputConfig { device } = board_config;

        daq.enter_config_mode(*device)?;
        let bcb = daq.create_acb(*device, InterfaceType::Output)?;
        let mut acb_cfg = DQACBCFG::empty();

        acb_cfg.scansz = 1;
        acb_cfg.framesize = frame_size;
        acb_cfg.frames = 4; // # of frames in circular buffer
        acb_cfg.mode = DQ_ACBMODE_RECYCLED;
        acb_cfg.dirflags = DQ_ACB_DIRECTION_OUTPUT | DQ_ACB_DATA_RAW; // TODO input

        let mut card_cfg = CFG405;
        let mut actual_freq = freq as f32;
        let mut num_channels = 1;

        // mutation
        parse_err!(DqAcbInitOps(
            bcb,
            &mut card_cfg,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            &mut actual_freq, // TODO figure out difference between CV and CL clocks
            &mut num_channels,
            vec![0].as_mut_ptr(),
            ptr::null_mut(),
            &mut acb_cfg
        ))?;

        let buffer_size = acb_cfg.framesize as usize;
        let output_buffer: Vec<u32> = (0..buffer_size)
            .map(|val| if val < buffer_size / 2 { 0xfff } else { 0 })
            .collect();

        let the_dio405 = Dio405 {
            daq,
            bcb,
            output_buffer,
        };

        parse_err!(DqeSetEvent(
            bcb,
            DQ_eFrameDone | DQ_ePacketLost | DQ_eBufferError | DQ_ePacketOOB | DQ_eBufferDone
        ))?;

        Ok(the_dio405)
    }

    pub(crate) fn trigger(&self) -> Result<(), PowerDnaError> {
        let mut copied_size: u32 = 0;
        let mut remaining_space: u32 = 0;

        let len = self.output_buffer.len() as u32;

        // TODO deal with failure
        parse_err!(DqAcbPutScansCopy(
            self.bcb,
            self.output_buffer.as_ptr() as *const i8,
            len,
            len,
            &mut copied_size,
            &mut remaining_space,
        ))?;

        Ok(())
    }
}

impl Bcb for Dio405 {
    fn bcb(&self) -> pDQBCB {
        self.bcb
    }
}

impl Drop for Dio405 {
    fn drop(&mut self) {
        match self.daq.destroy_acb(self.bcb) {
            Err(err) => {
                eprintln!("DqAcbDestroy failed. Error: {}", err);
            }
            Ok(_) => {}
        };
    }
}

unsafe impl Send for Dio405 {}
unsafe impl Sync for Dio405 {}

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
