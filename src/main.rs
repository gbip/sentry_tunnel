use sentry_tunnel::config::Config;
use sentry_tunnel::server::router;

use log::*;

fn main() {
    stderrlog::new()
        .verbosity(3)
        .modules([module_path!()])
        .init()
        .unwrap(); // Error, Warn and Info
    match Config::new_from_env_variables() {
        Ok(config) => {
            info!("{}", config);
            let addr = format!("{}:{}", config.ip, config.port);
            gotham::start(addr, router(&config.tunnel_path.clone(), config));
        }
        Err(e) => {
            error!("{}", e);
            std::process::exit(1)
        }
    }
}
