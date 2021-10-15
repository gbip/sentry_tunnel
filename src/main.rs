use gotham::handler::{HandlerError, HandlerResult};
use gotham::helpers::http::response::create_empty_response;
use gotham::hyper::{body, header, Body, HeaderMap, StatusCode};
use gotham::router::builder::{build_simple_router, DefineSingleRoute, DrawRoutes};
use gotham::router::Router;
use gotham::state::{FromState, State};

use std::error::Error;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

mod tunnel;

use tunnel::{make_error, RemoteSentryInstance};

// 1 MB max body
const MAX_CONTENT_SIZE: u16 = 1_000;

fn parse_body(body: String) -> Result<RemoteSentryInstance, HandlerError> {
    RemoteSentryInstance::try_new_from_body(body)
}

#[derive(Debug)]
#[non_exhaustive]
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
            HeaderError::ContentIsTooBig => f.write_str("Content length is too big"),
            _ => f.write_str("Invalid header"),
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
                        sentry_instance.forward("toto").await;
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
