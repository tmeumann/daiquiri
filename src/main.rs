mod powerdna;

use powerdna::{ DqEngine };

const IP: &str = "192.168.100.2";

fn main() {
    let mut engine = DqEngine::new(1000).expect("Failed to initialise DqEngine.");
    let daq = engine.open_daq(String::from(IP)).expect("Failed to open DAQ.");
    daq.configure_inputs(vec![0], 1000).expect("Failed to configure IO boards.");
    daq.stream();
    println!("descriptor: {}", daq.handle);
}
