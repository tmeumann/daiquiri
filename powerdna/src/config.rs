use serde::{Deserialize};

#[derive(Deserialize, Debug)]
pub struct StreamConfig {
    pub ip: String,
    pub freq: u32,
    pub frame_size: u32,
    pub board: BoardConfig,
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
