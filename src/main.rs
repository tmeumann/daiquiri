mod powerdna;
mod bootstrap;

use crate::powerdna::SignalManager;
use crate::powerdna::Daq;
use std::collections::HashMap;
use powerdna::{ DqEngine };
use std::sync::{Arc, Mutex};
use bootstrap::initialise;

use warp::Filter;

type SignalStore = Arc<HashMap<String, Mutex<SignalManager>>>;

#[tokio::main]
async fn main() {
    let signal_managers = initialise().expect("Failed to initialise DAQ threads.");

    let start = warp::path("start")
        .and(warp::path::param())
        .and(with_signal_manager(Arc::clone(&signal_managers)))
        .and_then(start_stream);

    let stop = warp::path("stop")
        .and(warp::path::param())
        .and(with_signal_manager(Arc::clone(&signal_managers)))
        .and_then(stop_stream);

    let cors = warp::cors().allow_any_origin().allow_method(warp::http::Method::POST);
    let routes = warp::post().and(start.or(stop)).with(cors);

    warp::serve(routes).run(([0, 0, 0, 0], 3030)).await;
}

fn with_signal_manager(store: SignalStore) -> impl Filter<Extract = (SignalStore,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || Arc::clone(&store))
}

async fn start_stream(topic: String, store: SignalStore) -> Result<impl warp::Reply, warp::Rejection> {
    match store.get(&topic) {
        // TODO return start timestamp
        Some(mutex) => match mutex.lock() {
            Ok(mut manager) => match manager.start() {
                Ok(_) => Ok(warp::reply()),
                Err(_) => Err(warp::reject::not_found()), // TODO
            },
            Err(_) => Err(warp::reject::not_found()),
        },
        None => Err(warp::reject::not_found()),
    }
}

async fn stop_stream(topic: String, store: SignalStore) -> Result<impl warp::Reply, warp::Rejection> {
    match store.get(&topic) {
        // TODO return start timestamp
        Some(mutex) => match mutex.lock() {
            Ok(mut manager) => match manager.stop() {
                Ok(_) => Ok(warp::reply()),
                Err(_) => Err(warp::reject::not_found()), // TODO
            },
            Err(_) => Err(warp::reject::not_found()),
        },
        None => Err(warp::reject::not_found()),
    }
}
