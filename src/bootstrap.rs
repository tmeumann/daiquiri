use crate::powerdna::{DaqError, SignalManager};
use crate::Daq;
use crate::DqEngine;
use std::sync::{Arc, Mutex};
use std::io::BufReader;
use std::fs::File;
use std::collections::HashMap;
use serde::{Deserialize};
use thiserror::Error;
use std::io;
use std::env;
use serde_json;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::ClientConfig;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedReceiver;

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
    #[error("Failed to connect to Kafka.")]
    KafkaError {
        #[from]
        source: rdkafka::error::KafkaError,
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

async fn publish(producer: FutureProducer, mut rx: UnboundedReceiver<(String, Vec<u8>)>) {
    // TODO clean pack-up
    loop {
        let (topic, data) = match rx.recv().await {
            Some(val) => {
                val
            },
            None => {
                break;
            },
        };
        match producer.send(
            FutureRecord::to(topic.as_str()).key(topic.as_str()).payload(&data),
            Duration::from_secs(180),
        ).await {
            Ok(_) => (),
            Err((err, _)) => eprintln!("Failed to send to Kafka. Error: {}", err),
        };
    }
}


pub fn initialise() -> Result<Arc<HashMap<String, Mutex<SignalManager>>>, ConfigError> {
    let clock_period: u32 = match env::var("CLOCK_PERIOD").unwrap_or(String::from("1000")).parse() {
        Ok(val) => val,
        Err(_) => return Err(ConfigError::InvalidClockPeriod),
    };
    let file_path = env::var("STREAM_CONFIG").unwrap_or(String::from("/etc/daiquiri/streams.json"));
    
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let mut config: HashMap<String, StreamConfig> = serde_json::from_reader(reader)?;

    // TODO validate config -- no repeated stream names, IPs, etc.

    let engine = Arc::new(DqEngine::new(clock_period)?);

    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", "host.docker.internal:19092")
        .set("message.timeout.ms", "5000")
        .create()?;

    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    println!("Spawning publisher...");
    tokio::spawn(async move {
        println!("wut");
        publish(producer, rx).await
    });

    let streams = config.drain()
        .map(|(name, config)| {
            let StreamConfig { ip, freq, board } = config;
            let daq = Arc::new(Daq::new(engine.clone(), ip.clone())?);
            let manager = Mutex::new(SignalManager::new(
                name.clone(),
                freq,
                board,
                daq,
                tx.clone(),
                None,
            ));
            Ok((name, manager))
        })
        .collect::<Result<HashMap<String, Mutex<SignalManager>>, DaqError>>()?;

    Ok(Arc::new(streams))
}
