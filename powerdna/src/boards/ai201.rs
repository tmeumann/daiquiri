use crate::boards::{Bcb, Empty, EVENT_TIMEOUT};
use crate::config::{BoardConfig, ChannelConfig};
use crate::daq::Daq;
use crate::engine::InterfaceType;
use crate::results::PowerDnaError;
use crate::DaqError;
use core::marker::{Send, Sync};
use core::mem;
use core::result::Result;
use core::result::Result::{Err, Ok};
use core::sync::atomic::{AtomicBool, Ordering};
use num_derive::ToPrimitive;
use num_traits::ToPrimitive;
use powerdna_sys::{
    pDATACONV, pDQBCB, DQ_eBufferDone, DQ_eBufferError, DQ_eFrameDone, DQ_ePacketLost,
    DQ_ePacketOOB, DqAcbGetScansCopy, DqAcbInitOps, DqConvRaw2ScalePdc, DqeSetEvent,
    DqeWaitForEvent, DQACBCFG, DQ_ACBMODE_CYCLE, DQ_ACB_DATA_RAW, DQ_ACB_DATA_TSCOPY,
    DQ_ACB_DIRECTION_INPUT, DQ_AI201_GAIN_10_100, DQ_AI201_GAIN_1_100, DQ_AI201_GAIN_2_100,
    DQ_AI201_GAIN_5_100, DQ_AI201_MODEFIFO, DQ_LNCL_TIMESTAMP, DQ_LN_ACTIVE, DQ_LN_CLCKSRC0,
    DQ_LN_ENABLED, DQ_LN_GETRAW, DQ_LN_IRQEN, DQ_LN_STREAMING,
};
use std::ptr;
use std::sync::mpsc::Sender;
use std::sync::Arc;

const CFG201: u32 = DQ_LN_ENABLED
    | DQ_LN_ACTIVE
    | DQ_LN_GETRAW
    | DQ_LN_IRQEN
    | DQ_LN_CLCKSRC0
    | DQ_LN_STREAMING
    | DQ_AI201_MODEFIFO;

pub struct Ai201 {
    bcb: pDQBCB,
    channels: Vec<u32>,
    pdc: pDATACONV,
    acb_cfg: DQACBCFG,
    daq: Arc<Daq>,
    buffer_size: usize,
    out: Sender<(Vec<f64>, Vec<u32>)>,
}

impl Ai201 {
    pub fn new(
        daq: Arc<Daq>,
        freq: u32,
        frame_size: u32,
        board_config: &BoardConfig,
        out: Sender<(Vec<f64>, Vec<u32>)>,
    ) -> Result<Self, DaqError> {
        let BoardConfig { device, channels } = board_config;
        daq.enter_config_mode(*device)?;
        let bcb = daq.create_acb(*device, InterfaceType::Input)?;

        let mut channel_list = channels
            .iter()
            .map(|ChannelConfig { id, gain }| -> Result<u32, DaqError> {
                let gain_as_u32 = ToPrimitive::to_u32(gain).ok_or(DaqError::GainConfigError)?;
                Ok(*id as u32 | (gain_as_u32 << 8))
            })
            .collect::<Result<Vec<u32>, DaqError>>()?;
        channel_list.push(0);
        channel_list.push(DQ_LNCL_TIMESTAMP);

        let mut acb_cfg = DQACBCFG::empty();

        acb_cfg.samplesz = mem::size_of::<u16>() as u32; // size of single reading
        acb_cfg.scansz = channel_list.len() as u32; // number of readings (timestamp is equivalent to 2 readings)
        acb_cfg.framesize = frame_size;
        acb_cfg.frames = 12; // # of frames in circular buffer TODO
        acb_cfg.mode = DQ_ACBMODE_CYCLE;
        acb_cfg.dirflags = DQ_ACB_DIRECTION_INPUT | DQ_ACB_DATA_RAW | DQ_ACB_DATA_TSCOPY;

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

    pub fn sample(&self, stop: Arc<AtomicBool>) {
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
                    Err(err) => {
                        eprintln!("Failed to send frame data to muxer thread. Error: {}", err);
                        break 'outer;
                    }
                };
            }
        }
    }

    fn wait_for_event(&self) -> Result<u32, PowerDnaError> {
        let mut events: u32 = 0;
        parse_err!(DqeWaitForEvent(&self.bcb, 1, 0, EVENT_TIMEOUT, &mut events))?;
        Ok(events)
    }

    fn extract_timestamps(&self, raw_buffer: &Vec<u16>, scans: usize) -> Vec<u32> {
        let mut timestamps: Vec<u32> = Vec::with_capacity(scans as usize);
        let num_chans = self.channels.len();

        for scan in 1..(scans + 1) {
            let upper_half = raw_buffer[(scan * num_chans) - 2] as u32; // TODO guard this
            let lower_half = raw_buffer[(scan * num_chans) - 1] as u32;
            let timestamp: u32 = (upper_half << 16) | lower_half;
            timestamps.push(timestamp);
        }

        timestamps
    }

    fn get_scaled_data(
        &self,
        raw_buffer: &mut Vec<u16>,
    ) -> Result<Vec<(Vec<f64>, Vec<u32>)>, PowerDnaError> {
        let framesize: u32 = self.acb_cfg.framesize;
        let mut received_scans: u32 = 0;
        let mut remaining_scans: u32 = 0;

        let buffer_ptr = raw_buffer.as_mut_ptr() as *mut i8;

        let mut scaled_frames: Vec<(Vec<f64>, Vec<u32>)> = vec![];

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

            let timestamps = self.extract_timestamps(raw_buffer, received_scans as usize);

            parse_err!(DqConvRaw2ScalePdc(
                self.pdc,
                self.channels.as_ptr(),
                chans,
                received_scans * chans,
                buffer_ptr,
                scaled_buffer.as_mut_ptr() as *mut f64
            ))?;

            scaled_frames.push((scaled_buffer, timestamps));

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
