mod powerdna;

use powerdna::{ DqEngine };

const IP: &str = "192.168.100.2";

fn main() {
    let mut engine = DqEngine::new(1000).expect("Failed to initialise DqEngine.");
    let daq = engine.open_daq(String::from(IP)).expect("Failed to open DAQ.");
    let _ = daq.add_201(0, 1000);
    println!("descriptor: {}", daq.handle);
}
