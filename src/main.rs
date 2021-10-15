use futures_util::future::{self, FutureExt};
use gotham::handler::{HandlerError, HandlerFuture, HandlerResult, IntoResponse};
use gotham::helpers::http::response::create_empty_response;
use gotham::hyper::{body, header, Body, HeaderMap, Method, Response, StatusCode, Uri, Version};
use gotham::router::builder::{build_simple_router, DefineSingleRoute, DrawRoutes};
use gotham::router::Router;
use gotham::state::{FromState, State};
use serde_json::Value;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::pin::Pin;
use std::str::FromStr;

mod tunnel;

use tunnel::RemoteSentryInstance;

// 1 MB max body
const MAX_CONTENT_SIZE: u16 = 1_000;

fn parse_body(body: String) -> Result<RemoteSentryInstance, HandlerError> {
    RemoteSentryInstance::try_new_from_body(body)
}

fn check_content_length(headers: &HeaderMap) -> Result<(), ()> {
    if let Some(content_length_value) = headers.get(header::CONTENT_LENGTH) {
        let content_length =
            u16::from_str(content_length_value.to_str().map_err(|_| ())?).map_err(|_| ())?;
        if content_length > MAX_CONTENT_SIZE {
            return Err(());
        } else {
            return Ok(());
        }
    }
    Err(())
}

/// Extracts the elements of the POST request and prints them
async fn post_tunnel_handler(mut state: State) -> HandlerResult {
    let header = HeaderMap::take_from(&mut state);
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
