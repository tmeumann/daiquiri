mod powerdna;

use crate::powerdna::Daq;
use std::sync::Mutex;
use crate::powerdna::Gain;
use std::collections::HashMap;
use powerdna::{ DqEngine };
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::env;

use serde::{Deserialize};

const IP: &str = "192.168.100.2";

use warp::Filter;

#[derive(Deserialize)]
struct StreamConfig {
    freq: u32,
    boards: HashMap<u32, Vec<u32>>,  // device number -> channels
    gain: Gain,
}

type SharedEngine = Arc<Mutex<DqEngine>>;

#[tokio::main]
async fn main() {
    let stop = Arc::new(AtomicBool::new(false));
    let r = stop.clone();
    ctrlc::set_handler(move || {
        r.store(true, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    let e = Arc::new(Mutex::new(DqEngine::new(String::from(IP), 1000).expect("Failed to initialise DqEngine")));
    
    let prepare = warp::path("prepare")
        .and(with_engine(Arc::clone(&e)))
        .and(with_stop(stop))
        .map(move |shared_engine: SharedEngine, stopper: Arc<AtomicBool>| {
            let mut engine = match shared_engine.lock() {
                Ok(eng) => eng,
                Err(_) => return warp::reply(),  // TODO 500
            };
            engine.configure_stream(0, 1000, stopper);
            warp::reply()
        });
    
    let start = warp::path("start")
        .and(with_engine(Arc::clone(&e)))
        .map(|shared_engine: SharedEngine| {
            let mut engine = match shared_engine.lock() {
                Ok(eng) => eng,
                Err(_) => return warp::reply(),  // TODO 500
            };
            engine.start_stream();
            warp::reply()
            // return start timestamp
        });

    let stop = warp::path("stop")
        .and(with_engine(Arc::clone(&e)))
        .map(|shared_engine: SharedEngine| {
            let mut engine = match shared_engine.lock() {
                Ok(eng) => eng,
                Err(_) => return warp::reply(),  // TODO 500
            };
            engine.stop_stream();
            warp::reply()
            // return stop timestamp
        });
    
    let routes = warp::post().and(prepare.or(start).or(stop));
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;

    // let stream = daq.stream(vec![0], 1000, stop).expect("Failed to configure IO boards").as_mut().unwrap();
    // for frame in stream {
    //     for i in 0..1000 {
    //         let start = i * 26;
    //         let end = start + 24; // ignore time stamps
    //         let slice = &frame[start..end];
    //         for val in slice {
    //             print!("{:.4} ", val);
    //         }
    //         println!();
    //     }
    // }
}

fn with_engine(engine: SharedEngine) -> impl Filter<Extract = (SharedEngine,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || Arc::clone(&engine))
}

fn with_stop(stop: Arc<AtomicBool>) -> impl Filter<Extract = (Arc<AtomicBool>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || Arc::clone(&stop))
}

// fn prepare(daq: Arc<Mutex<Daq>>) {
//     let daq = Arc::clone(&daq);
//     warp::path("prepare")
//         .and(warp::body::json())
//         .map(move |config: StreamConfig| {
            
//             // let stream = daq.stream(vec![0], 1000, stop).expect("Failed to configure IO boards").as_mut().unwrap();
//             warp::reply()
//         })
// }
