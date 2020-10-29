use powerdna::{DaqError, SignalManager, daq::Daq, engine::DqEngine, config::StreamConfig};
use std::sync::{Arc, Mutex};
use std::io::BufReader;
use std::fs::File;
use std::collections::HashMap;
use thiserror::Error;
use std::{io, thread, mem};
use std::env;
use serde_json;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::ClientConfig;
use std::time::Duration;
use std::sync::mpsc::{channel, Receiver};

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

fn publish(producer: FutureProducer, mut rx: Receiver<(String, Vec<u8>)>) {
    // TODO clean pack-up
    loop {
        let (topic, data) = match rx.recv() {
            Ok(val) => {
                val
            },
            Err(_) => {
                eprintln!("something broke in the channel...");
                break;
            },
        };
        println!("{}: {}", topic, data.len() / mem::size_of::<f64>());
        // match producer.send(
        //     FutureRecord::to(topic.as_str()).key(topic.as_str()).payload(&data),
        //     Duration::from_secs(180),
        // ).await {
        //     Ok(_) => (),
        //     Err((err, _)) => eprintln!("Failed to send to Kafka. Error: {}", err),
        // };
    }
}


pub fn initialise() -> Result<Arc<Mutex<HashMap<String, SignalManager>>>, ConfigError> {
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

    let (tx, rx) = channel();
    thread::spawn(move || {
        publish(producer, rx)
    });

    let streams = config.drain()
        .map(|(name, config)| {
            let StreamConfig { ip, freq, board } = config;
            let daq = Arc::new(Daq::new(engine.clone(), ip.clone())?);
            let manager = SignalManager::new(
                name.clone(),
                freq,
                board,
                daq,
                tx.clone(),
                None,
            );
            Ok((name, manager))
        })
        .collect::<Result<HashMap<String, SignalManager>, DaqError>>()?;

    Ok(Arc::new(Mutex::new(streams)))
}
