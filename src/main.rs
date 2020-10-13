mod powerdna;
mod bootstrap;

use crate::powerdna::SignalStream;
use crate::powerdna::Daq;
use std::collections::HashMap;
use powerdna::{ DqEngine };
use std::sync::Arc;
use bootstrap::initialise;

use warp::Filter;

type StreamStore = Arc<HashMap<String, SignalStream>>;

#[tokio::main]
async fn main() {
    let streams = initialise().expect("Failed to initialise DAQ threads.");
    
    let start = warp::path("start")
        .and(warp::path::param())
        .and(with_streams(Arc::clone(&streams)))
        .and_then(start_stream);

    let stop = warp::path("stop")
        .and(warp::path::param())
        .and(with_streams(Arc::clone(&streams)))
        .and_then(stop_stream);
    
    let routes = warp::post().and(start.or(stop));
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}

fn with_streams(streams: StreamStore) -> impl Filter<Extract = (StreamStore,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || Arc::clone(&streams))
}

async fn start_stream(topic: String, store: StreamStore) -> Result<impl warp::Reply, warp::Rejection> {
    match store.get(&topic) {
        // TODO return start timestamp
        Some(stream) => match stream.start() {
            Ok(_) => Ok(warp::reply()),
            Err(_) => Err(warp::reject::not_found()), // TODO
        },
        None => Err(warp::reject::not_found()),
    }
}

async fn stop_stream(topic: String, store: StreamStore) -> Result<impl warp::Reply, warp::Rejection> {
    match store.get(&topic) {
        // TODO return start timestamp
        Some(stream) => match stream.stop() {
            Ok(_) => Ok(warp::reply()),
            Err(_) => Err(warp::reject::not_found()), // TODO
        },
        None => Err(warp::reject::not_found()),
    }
}
