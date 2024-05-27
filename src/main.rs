#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

#[cfg(all(debug_assertions, feature = "hot-reload"))]
use std::sync::{Arc};

#[cfg(all(debug_assertions, feature = "hot-reload"))]
use tokio::sync::{Mutex, RwLock};
#[cfg(all(debug_assertions, feature = "hot-reload"))]
use tokio::{sync::mpsc};

#[cfg(all(debug_assertions, feature = "hot-reload"))]
mod observe;
mod path_info;
mod hot_libs;

mod main_task;

#[cfg(any(not(debug_assertions), not(feature = "hot-reload")))]
#[tokio::main]
async fn main() -> std::io::Result<()>  {
    main_task::run().await
}

#[cfg(all(debug_assertions, feature = "hot-reload"))]
#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Use RUST_LOG=hot_lib_reloader=trace to see all related logs
    env_logger::init();

    #[cfg(feature = "path-info")]
    path_info::print_paths();

    //this channel is to shut down the server 
    let (tx_shutdown_server, rx_shutdown_server) = mpsc::channel(1);
    let rx_shutdown_server = Arc::new(RwLock::new(rx_shutdown_server));

    //ensures that the server and reloads are blocking
    let block_reloads_mutex = Arc::new(Mutex::new(0));

    //this is mainly so we don't send messages to a dead server 
    let server_is_running = Arc::new(RwLock::new(false));
    let server_is_running_writer = server_is_running.clone();

    let block_reloads_mutex_task = block_reloads_mutex.clone();
    let server_is_running_reader = server_is_running.clone();

    tokio::task::spawn(async move {
        observe::run(
            server_is_running_reader,
            tx_shutdown_server,
            block_reloads_mutex_task).await
    });

    //main loop
    loop {
        //only run when we can access the mutex
        let lock = block_reloads_mutex.lock().await;

        println!("---main loop---");

        main_task::run(server_is_running_writer.clone(), rx_shutdown_server.clone()).await;

        println!("---main loop finished---");

        //only allow more reloads when we are finished
        drop(lock);
    }
}
