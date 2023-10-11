use anyhow::Error as AError;

use gotham::handler::HandlerResult;
use gotham::handler::IntoResponse;
use gotham::helpers::http::response::create_empty_response;
use gotham::helpers::http::response::create_response;
use gotham::hyper::{body, header, Body, HeaderMap, Response, StatusCode};
use gotham::middleware::state::StateMiddleware;
use gotham::pipeline::single::single_pipeline;
use gotham::pipeline::single_middleware;
use gotham::router::{
    builder::build_router, builder::DefineSingleRoute, builder::DrawRoutes, Router,
};
use gotham::state::{FromState, State};
use gotham_derive::StateData;

use log::*;

use mime::Mime;

use std::error::Error;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use std::sync::Arc;

use crate::config::Config;
use crate::envelope::{BodyError, SentryEnvelope};

// 10 MB max body
pub const MAX_CONTENT_SIZE: u64 = 10_000_000;

/**
 * This struct is used to share read-only data between HTTP request handlers
 */
#[derive(Debug, StateData, Clone)]
struct TunnelConfig {
    inner: Arc<Config>,
}

fn parse_body(body: String) -> Result<SentryEnvelope, AError> {
    SentryEnvelope::try_new_from_body(body)
}

/**
 * This enum reprensent an header parsing error
 */
#[derive(Debug)]
pub enum HeaderError {
    MissingContentLength,
    ContentIsTooBig,
    CouldNotParseContentLength,
    InvalidHost,
}

impl Error for HeaderError {}

impl Display for HeaderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            HeaderError::MissingContentLength => f.write_str("Missing content length header."),
            HeaderError::ContentIsTooBig => f.write_str("Content length too big."),
            HeaderError::CouldNotParseContentLength => {
                f.write_str("could not parse content length header.")
            }
            HeaderError::InvalidHost => f.write_str(
                "Invalid sentry host, check your config against the dsn used in the request.",
            ),
        }
    }
}

impl IntoResponse for HeaderError {
    fn into_response(self, state: &State) -> Response<Body> {
        warn!("{}", self);
        let mime = "text/plain".parse::<Mime>().unwrap();
        create_response(state, StatusCode::BAD_REQUEST, mime, format!("{}", self))
    }
}

/**
 * Returns Ok if the request associated with those headers can be handled
 */
fn check_content_length(headers: &HeaderMap) -> Result<(), AError> {
    if let Some(content_length_value) = headers.get(header::CONTENT_LENGTH) {
        let content_length = u64::from_str(
            content_length_value
                .to_str()
                .map_err(|_| AError::new(HeaderError::CouldNotParseContentLength))?,
        )
        .map_err(|_| AError::new(HeaderError::CouldNotParseContentLength))?;
        if content_length > MAX_CONTENT_SIZE {
            return Err(AError::new(HeaderError::ContentIsTooBig));
        } else {
            return Ok(());
        }
    }
    Err(AError::new(HeaderError::MissingContentLength))
}

async fn tunnel_handler(state: &mut State) -> Result<Response<Body>, AError> {
    let headers = HeaderMap::take_from(state);
    check_content_length(&headers)?;

    let full_body = body::to_bytes(Body::take_from(state)).await?;
    let body_content = String::from_utf8(full_body.to_vec())?;
    let sentry_instance = parse_body(body_content)?;

    let config = TunnelConfig::borrow_from(state);
    let hosts = &config.inner.remote_hosts;
    if config
        .inner
        .project_id_is_allowed(sentry_instance.dsn.project_id().value())
    {
        if sentry_instance.dsn_host_is_valid(hosts) {
            match sentry_instance.forward().await {
                Err(e) => {
                    error!(
                        "Failed to forward request to sentry : {} - Host = {}",
                        e,
                        sentry_instance.dsn.host()
                    );
                    let mime = "text/plain".parse::<Mime>().unwrap();
                    let res: (StatusCode, Mime, String) =
                        (StatusCode::INTERNAL_SERVER_ERROR, mime, format!("{}", e));
                    let res = res.into_response(state);
                    Ok(res)
                }
                Ok(_) => {
                    let res = create_empty_response(state, StatusCode::OK);
                    Ok(res)
                }
            }
        } else {
            Err(AError::new(HeaderError::InvalidHost))
        }
    } else {
        Err(AError::new(BodyError::InvalidProjectId))
    }
}

async fn post_tunnel_handler(mut state: State) -> HandlerResult {
    match tunnel_handler(&mut state).await {
        Ok(val) => Ok((state, val)),
        Err(error) => {
            let mime = "text/plain".parse::<Mime>().unwrap();
            let res: (StatusCode, Mime, String) = (
                StatusCode::BAD_REQUEST,
                mime,
                format!("{}", error),
            );
            let response = res.into_response(&state);
            Ok((state, response))
        }
    }
}


async fn health_handler(state: State) -> HandlerResult {
    let response = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/plain")
        .body(Body::from("OK"))
        .unwrap();
    Ok((state, response))
}

pub fn router(path: &str, config: Config) -> Router {
    let middleware = StateMiddleware::new(TunnelConfig {
        inner: Arc::new(config),
    });
    let pipeline = single_middleware(middleware);
    let (chain, pipelines) = single_pipeline(pipeline);

    build_router(chain, pipelines, |route| {
        route.post(path).to_async(post_tunnel_handler);
        route.get("/healthz").to_async(health_handler);
    })
}
