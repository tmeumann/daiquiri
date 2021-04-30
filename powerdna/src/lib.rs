use crate::config::{BoardConfig, OutputConfig};
use crate::daq::Daq;
use crate::stream::Sampler;
use powerdna_sys::DQ_AI201_GAIN_10_100;
use powerdna_sys::DQ_AI201_GAIN_1_100;
use powerdna_sys::DQ_AI201_GAIN_2_100;
use powerdna_sys::DQ_AI201_GAIN_5_100;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

#[macro_use]
mod results;

#[macro_use]
extern crate num_derive;

mod boards;
pub mod config;
pub mod daq;
pub mod engine;
mod stream;

use serde::de::Visitor;
use serde::{de, Deserialize, Deserializer};
use std::fmt;
use std::fmt::Formatter;
use std::prelude::v1::Result::Ok;

#[derive(Debug, ToPrimitive)]
#[repr(u32)]
pub enum Gain {
    One = DQ_AI201_GAIN_1_100,
    Two = DQ_AI201_GAIN_2_100,
    Five = DQ_AI201_GAIN_5_100,
    Ten = DQ_AI201_GAIN_10_100,
}

struct GainVisitor;

impl<'de> Visitor<'de> for GainVisitor {
    type Value = Gain;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("the integers 1, 2, 5 or 10")
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match value {
            1 => Ok(Gain::One),
            2 => Ok(Gain::Two),
            5 => Ok(Gain::Five),
            10 => Ok(Gain::Ten),
            _ => Err(E::custom(format!("expected 1, 2, 5 or 10: {}", value))),
        }
    }
}

impl<'de> Deserialize<'de> for Gain {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_i32(GainVisitor)
    }
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
    #[error("Unexpected gain value.")]
    GainConfigError,
}

pub struct SignalManager {
    name: String,
    freq: u32,
    frame_size: u32,
    boards: Vec<BoardConfig>,
    outputs: Vec<OutputConfig>,
    sampler: Option<Sampler>,
    out: UnboundedSender<(String, Vec<f64>, Vec<u32>)>,
    buzzer_out: UnboundedSender<(String, u32)>,
    daq: Arc<Daq>,
}

impl SignalManager {
    pub fn new(
        name: String,
        freq: u32,
        frame_size: u32,
        boards: Vec<BoardConfig>,
        outputs: Vec<OutputConfig>,
        daq: Arc<Daq>,
        out: UnboundedSender<(String, Vec<f64>, Vec<u32>)>,
        buzzer_out: UnboundedSender<(String, u32)>,
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
            buzzer_out,
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
                    self.buzzer_out.clone(),
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

    pub async fn trigger(&mut self) -> Result<(), DaqError> {
        match &mut self.sampler {
            Some(sampler) => {
                sampler.trigger().await?;
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
