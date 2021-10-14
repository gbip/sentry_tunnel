use futures_util::future::{self, FutureExt};
use gotham::hyper::{body, Body, HeaderMap, Method, Response, StatusCode, Uri, Version};
use std::pin::Pin;

use gotham::handler::{HandlerError, HandlerFuture, HandlerResult, IntoResponse};
use gotham::helpers::http::response::create_empty_response;
use gotham::router::builder::{build_simple_router, DefineSingleRoute, DrawRoutes};
use gotham::router::Router;
use gotham::state::{FromState, State};
use serde_json::Value;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

#[derive(Debug)]
struct RemoteSentryInstance {
    project_id: String,
    raw_body: String,
}

#[derive(Debug)]
enum BodyError {
    MalformedBody,
    InvalidHeaderJson(serde_json::Error),
    MissingDsnKeyInHeader,
}

impl Display for BodyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BodyError::MalformedBody => f.write_str("Malformed HTTP Body"),
            BodyError::MissingDsnKeyInHeader => f.write_str("dsn key was not found in header"),
            BodyError::InvalidHeaderJson(e) => f.write_fmt(format_args!("{}", e)),
            _ => f.write_str("Invalid body"),
        }
    }
}

impl Error for BodyError {}

fn make_error(err: BodyError) -> HandlerError {
    HandlerError::from(err).with_status(StatusCode::BAD_REQUEST)
}

impl RemoteSentryInstance {
    async fn forward(self, host: &str) {
        println!("Forwarding {} to {}", self.raw_body, host);
        let _reponse = isahc::post_async(host, self.raw_body);
    }

    fn try_new_from_body(body: String) -> Result<RemoteSentryInstance, HandlerError> {
        if body.lines().count() == 3 {
            let header = body
                .lines()
                .next()
                .ok_or(make_error(BodyError::MalformedBody))?;
            let header: Value = serde_json::from_str(header).map_err(|e| {
                make_error(BodyError::InvalidHeaderJson(e)).with_status(StatusCode::BAD_REQUEST)
            })?;
            if let Some(dsn) = header.get("dsn") {
                if let Some(dsn_str) = dsn.as_str() {
                    let (_url, project_id) = dsn_str
                        .rsplit_once('/')
                        .ok_or(make_error(BodyError::MalformedBody))?;
                    Ok(RemoteSentryInstance {
                        project_id: project_id.to_string(),
                        raw_body: body,
                    })
                } else {
                    Err(make_error(BodyError::MalformedBody))
                }
            } else {
                Err(make_error(BodyError::MissingDsnKeyInHeader))
            }
        } else {
            Err(make_error(BodyError::MalformedBody))
        }
    }
}

fn parse_body(body: String) -> Result<RemoteSentryInstance, HandlerError> {
    RemoteSentryInstance::try_new_from_body(body)
}

/// Extracts the elements of the POST request and prints them
async fn post_tunnel_handler(mut state: State) -> HandlerResult {
    // Check content length
    let full_body = body::to_bytes(Body::take_from(&mut state)).await;
    match full_body {
        Ok(valid_body) => {
            let body_content = String::from_utf8(valid_body.to_vec()).unwrap();
            match parse_body(body_content) {
                Ok(sentry_instance) => {
                    sentry_instance.forward("toto").await;
                    let res = create_empty_response(&state, StatusCode::OK);
                    Ok((state, res))
                }
                Err(e) => Err((state, e)),
            }
        }
        Err(e) => Err((state, e.into())),
    }
}
fn router(path: &str) -> Router {
    build_simple_router(|route| {
        route.post(path).to_async(post_tunnel_handler);
    })
}

fn main() {
    let addr = "127.0.0.1:7878";
    let tunnel_uri = "/tunnel";
    println!("Listening for requests at http://{}", addr);
    gotham::start(addr, router(tunnel_uri));
}
