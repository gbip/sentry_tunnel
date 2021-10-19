use futures_util::future::{self, Either, FutureExt};
use log::*;
use sentry_tunnel::config::Config;
use sentry_tunnel::server::router;
use tokio::signal;

#[tokio::main]
pub async fn main() {
    stderrlog::new()
        .verbosity(3)
        .modules([module_path!()])
        .init()
        .unwrap(); // Error, Warn and Info

    match Config::new_from_env_variables() {
        Ok(config) => {
            info!("{}", config);
            let addr = format!("{}:{}", config.ip, config.port);
            let signal = async {
                signal::ctrl_c().await.expect("failed to listen for event");
                println!("Ctrl+C pressed");
            };

            let server = gotham::init_server(addr, move || {
                Ok(router(&config.tunnel_path.clone(), config.clone()))
            });
            let res = future::select(server.boxed(), signal.boxed()).await;
            if let Either::Left((Err(err), _)) = res {
                println!("Error starting gotham: {:?}", err);
            } else {
                println!("Shutting down gracefully");
            }
        }
        Err(e) => {
            error!("{}", e);
            std::process::exit(1)
        }
    }
}
