use bootstrap::initialise;
use powerdna::SignalManager;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::signal;
use tokio::sync::Mutex;
use warp::Filter;

mod bootstrap;

#[allow(dead_code, unused_imports)]
#[path = "../target/flatbuffers/dataframe_generated.rs"]
mod dataframe_generated;

type SignalStore = Arc<Mutex<HashMap<String, SignalManager>>>;

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

    let buzzer = warp::path("buzzer")
        .and(warp::path::param())
        .and(with_signal_manager(Arc::clone(&signal_managers)))
        .and_then(trigger_buzzer);

    let cors = warp::cors()
        .allow_any_origin()
        .allow_method(warp::http::Method::POST);
    let routes = warp::post().and(start.or(stop).or(buzzer)).with(cors);

    let (_, server) =
        warp::serve(routes).bind_with_graceful_shutdown(([0, 0, 0, 0], 3030), async move {
            match signal::ctrl_c().await {
                Err(_) => eprintln!("Failed waiting for ^C"),
                Ok(_) => signal_managers.lock().await.clear(),
            };
        });

    server.await;
}

fn with_signal_manager(
    store: SignalStore,
) -> impl Filter<Extract = (SignalStore,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || Arc::clone(&store))
}

async fn start_stream(
    topic: String,
    store: SignalStore,
) -> Result<impl warp::Reply, warp::Rejection> {
    match store.lock().await.get_mut(&topic) {
        Some(manager) => match manager.start() {
            Ok(_) => Ok(warp::reply()),
            Err(_) => Err(warp::reject::not_found()), // TODO
        },
        None => Err(warp::reject::not_found()),
    }
}

async fn stop_stream(
    topic: String,
    store: SignalStore,
) -> Result<impl warp::Reply, warp::Rejection> {
    match store.lock().await.get_mut(&topic) {
        Some(manager) => match manager.stop() {
            Ok(_) => Ok(warp::reply()),
            Err(_) => Err(warp::reject::not_found()), // TODO
        },
        None => Err(warp::reject::not_found()),
    }
}

async fn trigger_buzzer(
    topic: String,
    store: SignalStore,
) -> Result<impl warp::Reply, warp::Rejection> {
    match store.lock().await.get_mut(&topic) {
        Some(manager) => match manager.trigger().await {
            Ok(_) => Ok(warp::reply()),
            Err(_) => Err(warp::reject::not_found()),
        },
        None => Err(warp::reject::not_found()),
    }
}
