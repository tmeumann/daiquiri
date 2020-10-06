mod powerdna;

use powerdna::{ DqEngine };
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const IP: &str = "192.168.100.2";

fn main() {
    let stop = Arc::new(AtomicBool::new(false));
    let r = stop.clone();

    ctrlc::set_handler(move || {
        r.store(true, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    let mut engine = DqEngine::new(1000).expect("Failed to initialise DqEngine");
    let daq = engine.open_daq(String::from(IP)).expect("Failed to open DAQ");
    let stream = daq.stream(vec![0], 1000, stop).expect("Failed to configure IO boards").as_mut().unwrap();
    for frame in stream {
        for i in 0..1000 {
            let start = i * 26;
            let end = start + 24; // ignore time stamps
            let slice = &frame[start..end];
            for val in slice {
                print!("{:.4} ", val);
            }
            println!();
        }
    }
}
