use serde::{Deserialize};

#[derive(Deserialize, Debug)]
pub struct StreamConfig {
    pub ip: String,
    pub freq: u32,
    pub frame_size: u32,
    pub boards: Vec<BoardConfig>,
    pub outputs: Vec<OutputConfig>,
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

#[derive(Deserialize, Debug)]
pub struct OutputConfig {
    pub device: u8,
}
