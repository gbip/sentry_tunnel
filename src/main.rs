use gotham::handler::{HandlerError, HandlerResult};
use gotham::helpers::http::response::create_empty_response;
use gotham::hyper::{body, header, Body, HeaderMap, StatusCode};
use gotham::middleware::state::StateMiddleware;
use gotham::pipeline::single::single_pipeline;
use gotham::pipeline::single_middleware;
use gotham::router::{
    builder::build_router, builder::DefineSingleRoute, builder::DrawRoutes, Router,
};
use gotham::state::{FromState, State};

use gotham_derive::StateData;

use std::error::Error;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use std::sync::Arc;

use log::*;

mod config;
mod tunnel;

use config::Config;
use tunnel::{make_error, RemoteSentryInstance};

// 1 MB max body
const MAX_CONTENT_SIZE: u16 = 1_000;

#[derive(Debug, StateData, Clone)]
struct TunnelConfig {
    inner: Arc<Config>,
}

fn parse_body(body: String) -> Result<RemoteSentryInstance, HandlerError> {
    RemoteSentryInstance::try_new_from_body(body)
}

#[derive(Debug)]
enum HeaderError {
    MissingContentLength,
    ContentIsTooBig,
    CouldNotParseContentLength,
}

impl Error for HeaderError {}

impl Display for HeaderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            HeaderError::MissingContentLength => f.write_str("Missing content length header"),
            HeaderError::ContentIsTooBig => f.write_str("Content length too big"),
            HeaderError::CouldNotParseContentLength => f.write_str("could not parse content length header"),
        }
    }
}

fn check_content_length(headers: &HeaderMap) -> Result<(), HeaderError> {
    if let Some(content_length_value) = headers.get(header::CONTENT_LENGTH) {
        let content_length = u16::from_str(
            content_length_value
                .to_str()
                .map_err(|_| HeaderError::CouldNotParseContentLength)?,
        )
        .map_err(|_| HeaderError::CouldNotParseContentLength)?;
        if content_length > MAX_CONTENT_SIZE {
            return Err(HeaderError::ContentIsTooBig);
        } else {
            return Ok(());
        }
    }
    Err(HeaderError::MissingContentLength)
}

/// Extracts the elements of the POST request and prints them
async fn post_tunnel_handler(mut state: State) -> HandlerResult {
    let headers = HeaderMap::take_from(&mut state);
    let full_body = body::to_bytes(Body::take_from(&mut state)).await;
    match full_body {
        Ok(valid_body) => {
            let body_content = String::from_utf8(valid_body.to_vec()).unwrap();
            match check_content_length(&headers) {
                Ok(_) => match parse_body(body_content) {
                    Ok(sentry_instance) => {
                        let config = TunnelConfig::borrow_from(&state);
                        let host = &config.inner.remote_host;
                        if let Err(e) = sentry_instance.forward(host).await {
                            error!("{} - Host = {}", e, host);
                        }
                        let res = create_empty_response(&state, StatusCode::OK);
                        Ok((state, res))
                    }
                    Err(e) => Err((state, e)),
                },
                Err(e) => Err((state, make_error(e))),
            }
        }
        Err(e) => Err((state, e.into())),
    }
}
fn router(path: &str, config: Config) -> Router {
    let middleware = StateMiddleware::new(TunnelConfig {
        inner: Arc::new(config),
    });
    let pipeline = single_middleware(middleware);
    let (chain, pipelines) = single_pipeline(pipeline);

    build_router(chain, pipelines, |route| {
        route.post(path).to_async(post_tunnel_handler);
    })
}

fn main() {
    stderrlog::new()
        .verbosity(2)
        .module(module_path!())
        .init()
        .unwrap(); // Error, Warn and Info
    match config::Config::new_from_env_variables() {
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
