use powerdna::SignalManager;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use bootstrap::initialise;
use std::thread;
use std::time::Duration;

mod bootstrap;

type SignalStore = Arc<Mutex<HashMap<String, SignalManager>>>;

fn main() {
    let signal_managers = initialise().expect("Failed to initialise DAQ threads.");

    match signal_managers.lock() {
        Ok(mut map) => match map.get_mut("V") {
            Some(manager) => match manager.start() {
                Ok(_) => println!("started"),
                Err(_) => eprintln!("Failed to start"), // TODO
            },
            None => eprintln!("Invalid topic?"),
        },
        Err(_) => eprintln!("Failed to lock..."),
    };

    thread::sleep(Duration::from_secs(1200));

    match signal_managers.lock() {
        Ok(mut map) => match map.get_mut("V") {
            Some(manager) => match manager.stop() {
                Ok(_) => println!("stopped"),
                Err(_) => eprintln!("Failed to start"), // TODO
            },
            None => eprintln!("Invalid topic?"),
        },
        Err(_) => eprintln!("Failed to lock..."),
    };

    // let start = warp::path("start")
    //     .and(warp::path::param())
    //     .and(with_signal_manager(Arc::clone(&signal_managers)))
    //     .and_then(start_stream);
    //
    // let stop = warp::path("stop")
    //     .and(warp::path::param())
    //     .and(with_signal_manager(Arc::clone(&signal_managers)))
    //     .and_then(stop_stream);

    // let cors = warp::cors().allow_any_origin().allow_method(warp::http::Method::POST);
    // let routes = warp::post().and(start.or(stop)).with(cors);
    //
    // let (_, server) = warp::serve(routes).bind_with_graceful_shutdown(([0, 0, 0, 0], 3030), async move {
    //     match signal::ctrl_c().await {
    //         Err(_) => eprintln!("Failed waiting for ^C"),
    //         Ok(_) => match signal_managers.lock() {
    //             Ok(mut map) => {
    //                 map.clear();
    //             },
    //             Err(_) => eprintln!("Failed to shut down gracefully."),
    //         },
    //     };
    // });
    //
    // server.await;
}

// fn with_signal_manager(store: SignalStore) -> impl Filter<Extract = (SignalStore,), Error = std::convert::Infallible> + Clone {
//     warp::any().map(move || Arc::clone(&store))
// }

// async fn start_stream(topic: String, store: SignalStore) -> Result<impl warp::Reply, warp::Rejection> {
//     match store.lock() {
//         Ok(mut map) => match map.get_mut(&topic) {
//             Some(manager) => match manager.start() {
//                 Ok(_) => Ok(warp::reply()),
//                 Err(_) => Err(warp::reject::not_found()), // TODO
//             },
//             None => Err(warp::reject::not_found()),
//         },
//         Err(_) => Err(warp::reject::not_found()),
//     }
// }
//
// async fn stop_stream(topic: String, store: SignalStore) -> Result<impl warp::Reply, warp::Rejection> {
//     match store.lock() {
//         Ok(mut map) => match map.get_mut(&topic) {
//             Some(manager) => match manager.stop() {
//                 Ok(_) => Ok(warp::reply()),
//                 Err(_) => Err(warp::reject::not_found()), // TODO
//             },
//             None => Err(warp::reject::not_found()),
//         },
//         Err(_) => Err(warp::reject::not_found()),
//     }
// }
