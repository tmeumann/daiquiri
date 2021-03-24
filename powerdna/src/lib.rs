use crate::config::{BoardConfig, OutputConfig};
use crate::daq::Daq;
use crate::stream::Sampler;
use powerdna_sys::DQ_AI201_GAIN_10_100;
use powerdna_sys::DQ_AI201_GAIN_1_100;
use powerdna_sys::DQ_AI201_GAIN_2_100;
use powerdna_sys::DQ_AI201_GAIN_5_100;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::mpsc::UnboundedSender;

#[macro_use]
mod results;

mod boards;
pub mod config;
pub mod daq;
pub mod engine;
mod stream;

#[derive(Debug)]
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
        source: results::PowerDnaError,
    },
    #[error("Invalid state for this action.")]
    StreamStateError,
    #[error("Unexpected number of channels.")]
    ChannelConfigError,
}

pub struct SignalManager {
    name: String,
    freq: u32,
    frame_size: u32,
    boards: Vec<BoardConfig>,
    outputs: Vec<OutputConfig>,
    daq: Arc<Daq>,
    out: UnboundedSender<(String, Vec<f64>)>,
    sampler: Option<Sampler>,
}

impl SignalManager {
    pub fn new(
        name: String,
        freq: u32,
        frame_size: u32,
        boards: Vec<BoardConfig>,
        outputs: Vec<OutputConfig>,
        daq: Arc<Daq>,
        out: UnboundedSender<(String, Vec<f64>)>,
        sampler: Option<Sampler>,
    ) -> Self {
        SignalManager {
            name,
            freq,
            frame_size,
            boards,
            outputs,
            daq,
            out,
            sampler,
        }
    }

    pub fn start(&mut self) -> Result<(), DaqError> {
        match self.sampler {
            Some(_) => Err(DaqError::StreamStateError),
            None => {
                let sampler = match Sampler::new(
                    self.daq.clone(),
                    self.freq,
                    self.frame_size,
                    &self.boards,
                    &self.outputs,
                    self.out.clone(),
                    self.name.clone(),
                ) {
                    Ok(sampler) => sampler,
                    Err(err) => {
                        println!("{:?}", err);
                        return Err(err);
                    }
                };
                self.sampler = Some(sampler);
                Ok(())
            }
        }
    }

    pub fn trigger(&mut self) -> Result<(), DaqError> {
        match &mut self.sampler {
            Some(sampler) => {
                sampler.trigger()?;
                Ok(())
            }
            None => Err(DaqError::StreamStateError),
        }
    }

    pub fn stop(&mut self) -> Result<(), DaqError> {
        match self.sampler {
            Some(_) => {
                self.sampler = None;
                Ok(())
            }
            None => Err(DaqError::StreamStateError),
        }
    }
}
