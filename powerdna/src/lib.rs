use powerdna_sys::DQ_AI201_GAIN_1_100;
use powerdna_sys::DQ_AI201_GAIN_2_100;
use powerdna_sys::DQ_AI201_GAIN_5_100;
use powerdna_sys::DQ_AI201_GAIN_10_100;
use std::sync::Arc;
use thiserror::Error;
use crate::config::BoardConfig;
use crate::daq::Daq;
use crate::stream::Sampler;
use tokio::sync::mpsc::UnboundedSender;

#[macro_use]
mod results;

pub mod engine;
mod boards;
pub mod daq;
mod stream;
pub mod config;

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
}


pub struct SignalManager {
    name: String,
    freq: u32,
    board: BoardConfig,
    daq: Arc<Daq>,
    out: UnboundedSender<(String, Vec<u8>)>,
    sampler: Option<Sampler>,
}


impl SignalManager {
    pub fn new(name: String, freq: u32, board: BoardConfig, daq: Arc<Daq>, out: UnboundedSender<(String, Vec<u8>)>, sampler: Option<Sampler>) -> Self {
        SignalManager {
            name,
            freq,
            board,
            daq,
            out,
            sampler,
        }
    }

    pub fn start(&mut self) -> Result<(), DaqError> {
        match self.sampler {
            Some(_) => Err(DaqError::StreamStateError),
            None => {
                self.sampler = Some(
                    Sampler::new(self.daq.clone(), self.freq, &self.board, self.out.clone(), self.name.clone())?
                );
                Ok(())
            }
        }
    }

    pub fn stop(&mut self) -> Result<(), DaqError> {
        match self.sampler {
            Some(_) => {
                self.sampler = None;
                Ok(())
            },
            None => Err(DaqError::StreamStateError)
        }
    }
}


