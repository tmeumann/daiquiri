use std::sync::mpsc::Receiver;
use zmq::Socket;
use std::sync::mpsc::channel;
use crate::powerdna::DaqError;
use crate::Daq;
use crate::DqEngine;
use std::sync::Arc;
use std::io::BufReader;
use std::fs::File;
use crate::powerdna::SignalStream;
use std::collections::HashMap;
use serde::{Deserialize};
use thiserror::Error;
use std::io;
use std::env;
use serde_json;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Couldn't open config file.")]
    FileError {
        #[from]
        source: io::Error,
    },
    #[error("Failed to parse config file.")]
    ParseError {
        #[from]
        source: serde_json::Error,
    },
    #[error("CLOCK_PERIOD invalid.")]
    InvalidClockPeriod,
    #[error("Failed to initialise DAQ.")]
    DaqInitialisationError {
        #[from]
        source: DaqError,
    },
    #[error("Failed to bind ZMQ socket.")]
    ZmqSocketError {
        #[from]
        source: zmq::Error,
    },
}

#[derive(Deserialize, Debug)]
struct StreamConfig {
    ip: String,
    freq: u32,
    board: BoardConfig,
}

#[derive(Deserialize, Debug)]
pub struct BoardConfig {
    pub device: u8,
    pub channels: Vec<ChannelConfig>,
}

#[derive(Deserialize, Debug)]
pub struct ChannelConfig {
    pub id: u8,
    pub gain: u32,  // TODO enum here
}

fn publish(socket: Socket, rx: Receiver<(String, Vec<u8>)>) {
    loop {
        let (topic, data) = match rx.recv() {
            Ok(val) => val,
            Err(_) => {
                break;
            },
        };
        socket.send(topic.as_str(), zmq::SNDMORE).unwrap();
        socket.send(data, 0).unwrap();
    }
}

pub fn initialise() -> Result<Arc<HashMap<String, SignalStream>>, ConfigError> {
    let clock_period: u32 = match env::var("CLOCK_PERIOD").unwrap_or(String::from("1000")).parse() {
        Ok(val) => val,
        Err(_) => return Err(ConfigError::InvalidClockPeriod),
    };
    let file_path = env::var("STREAM_CONFIG").unwrap_or(String::from("/etc/daiquiri/streams.json"));
    
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let config: HashMap<String, StreamConfig> = serde_json::from_reader(reader)?;

    // TODO validate config -- no repeated stream names, IPs, etc.

    let engine = Arc::new(DqEngine::new(clock_period).expect("Failed to initialise DqEngine"));

    let ctx = zmq::Context::new();
    let socket = ctx.socket(zmq::PUB).unwrap();
    socket.bind("tcp://*:5555")?;

    let (tx, rx) = channel();
    std::thread::spawn(move || {
        publish(socket, rx)
    });

    let streams = config.iter()
        .map(|(name, stream_config)| {
            let StreamConfig { ip, freq, board } = stream_config;
            let daq = Daq::new(engine.clone(), ip.clone())?;
            let stream = SignalStream::new(daq, *freq, board, tx.clone(), name.clone())?;
            Ok((name.clone(), stream))
        })
        .collect::<Result<HashMap<String, SignalStream>, DaqError>>()?;

    Ok(Arc::new(streams))
}
