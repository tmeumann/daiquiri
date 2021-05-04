use crate::dataframe_generated::daiquiri::{
    BuzzerEvent, BuzzerEventArgs, DaiquiriData, DaiquiriDataArgs, Event, SensorFrame,
    SensorFrameArgs,
};
use flatbuffers;
use powerdna::{config::StreamConfig, daq::Daq, engine::DqEngine, DaqError, SignalManager};
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::ClientConfig;
use serde_json;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io;
use std::io::BufReader;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::Mutex;

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

async fn publish_buzzer_events(
    producer: &FutureProducer,
    mut rx: UnboundedReceiver<(String, u32)>,
) {
    loop {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let (topic, timestamp) = match rx.recv().await {
            Some(val) => val,
            None => break,
        };
        let buzzer_event = BuzzerEvent::create(&mut builder, &BuzzerEventArgs { timestamp });
        let data = DaiquiriData::create(
            &mut builder,
            &DaiquiriDataArgs {
                event_type: Event::BuzzerEvent,
                event: Some(buzzer_event.as_union_value()),
            },
        );
        builder.finish(data, None);
        match producer
            .send(
                FutureRecord::to(topic.as_str())
                    .key(topic.as_str())
                    .payload(builder.finished_data()),
                Duration::from_secs(180),
            )
            .await
        {
            Ok(_) => (),
            Err((err, _)) => eprintln!("Failed to send to Kafka. Error: {}", err),
        };
    }
}

async fn publish_sensor_data(
    producer: &FutureProducer,
    mut rx: UnboundedReceiver<(String, Vec<f64>, Vec<u32>)>,
) {
    loop {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let (topic, data, timestamps) = match rx.recv().await {
            Some(val) => val,
            None => break,
        };
        let timestamps = Some(builder.create_vector(timestamps.as_slice()));
        let frame = Some(builder.create_vector(data.as_slice()));
        let sensor_frame = SensorFrame::create(
            &mut builder,
            &SensorFrameArgs {
                timestamps,
                frame,
                ..Default::default()
            },
        );
        let data = DaiquiriData::create(
            &mut builder,
            &DaiquiriDataArgs {
                event_type: Event::SensorFrame,
                event: Some(sensor_frame.as_union_value()),
            },
        );
        builder.finish(data, None);
        match producer
            .send(
                FutureRecord::to(topic.as_str())
                    .key(topic.as_str())
                    .payload(builder.finished_data()),
                Duration::from_secs(180),
            )
            .await
        {
            Ok(_) => (),
            Err((err, _)) => eprintln!("Failed to send to Kafka. Error: {}", err),
        };
    }
}

pub fn initialise() -> Result<Arc<Mutex<HashMap<String, SignalManager>>>, ConfigError> {
    let clock_period: u32 = match env::var("CLOCK_PERIOD")
        .unwrap_or(String::from("1000"))
        .parse()
    {
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

    let (sensor_tx, sensor_rx) = tokio::sync::mpsc::unbounded_channel();
    let sensor_producer = producer.clone();
    tokio::spawn(async move { publish_sensor_data(&sensor_producer, sensor_rx).await });
    let (buzzer_tx, buzzer_rx) = tokio::sync::mpsc::unbounded_channel();
    tokio::spawn(async move { publish_buzzer_events(&producer, buzzer_rx).await });

    let streams = config
        .drain()
        .map(|(name, config)| {
            let StreamConfig {
                ip,
                freq,
                frame_size,
                boards,
                outputs,
            } = config;
            let daq = Arc::new(Daq::new(engine.clone(), ip.clone())?);
            let manager = SignalManager::new(
                name.clone(),
                freq,
                frame_size,
                boards,
                outputs,
                daq,
                sensor_tx.clone(),
                buzzer_tx.clone(),
                None,
            );
            Ok((name, manager))
        })
        .collect::<Result<HashMap<String, SignalManager>, DaqError>>()?;

    Ok(Arc::new(Mutex::new(streams)))
}
