use crate::config::Host;
use gotham::anyhow::Error as AError;
use gotham::handler::IntoResponse;
use gotham::helpers::http::response::create_response;
use gotham::hyper::StatusCode;
use gotham::hyper::{body::Body, Response};
use gotham::state::State;
use isahc::{Request, RequestExt};
use mime::Mime;
use sentry_types::Dsn;
use serde_json::Value;

use log::*;

use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;

/**
 * Represent a sentry envelope
 */
#[derive(Debug)]
pub struct SentryEnvelope {
    pub raw_body: Vec<u8>,
    pub dsn: Dsn,
    pub is_safe: bool,
}

/**
 * A body parsing error
 */
#[derive(Debug)]
pub enum BodyError {
    InvalidNumberOfLines,
    InvalidHeaderJson(serde_json::Error),
    MissingDsnKeyInHeader,
    InvalidDsnValue,
    InvalidProjectId,
}

impl Display for BodyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BodyError::InvalidNumberOfLines => {
                f.write_str("Invalid number of line in request body. Should be 3.")
            }
            BodyError::MissingDsnKeyInHeader => {
                f.write_str("The dsn key is missing from the header header")
            }
            BodyError::InvalidHeaderJson(e) => {
                f.write_fmt(format_args!("Failed to parse header json : {}", e))
            }
            BodyError::InvalidProjectId => f.write_str("Unauthorized project ID"),
            BodyError::InvalidDsnValue => f.write_str("Failed to parse dsn value"),
        }
    }
}

impl Error for BodyError {}

impl IntoResponse for BodyError {
    fn into_response(self, state: &State) -> Response<Body> {
        warn!("{}", self);
        let mime = "application/json".parse::<Mime>().unwrap();
        create_response(state, StatusCode::BAD_REQUEST, mime, format!("{}", self))
    }
}

impl SentryEnvelope {
    /**
     * Returns true if this envelope is for an host that we are allowed to forward requests to
     */
    pub fn dsn_host_is_valid(&self, host: &[Host]) -> bool {
        let envelope_host = self.dsn.host().to_string();
        host.iter()
            .any(|x| x.0 == envelope_host)
    }

    /**
     * Forward this envelope to the destination sentry relay
     */
    pub async fn forward(&self) -> Result<(), AError> {
        let uri = self.dsn.envelope_api_url().to_string() + "?sentry_key=" + self.dsn.public_key();
        let request = Request::builder()
            .uri(uri)
            .header("Content-type", "application/x-sentry-envelope")
            .method("POST")
            .body(if !self.is_safe {
                self.raw_body.clone()
            } else {
                String::from_utf8(self.raw_body.clone()).unwrap().into_bytes()
            })?;
        info!(
            "Sending HTTP {} {} - body={}",
            request.method(),
            request.uri(),
            // request.body().len() if not is_safe else request.body()
            if !self.is_safe {
                format!("<{} bytes>", request.body().len())
            } else {
                // if request.body().len() is 0
                if request.body().is_empty() {
                   // print self.raw_body
                     format!("{:?}", self.raw_body)
                } else {
                    // format!("{:?}", request.body())
                    format!("<{} bytes> (safe)", request.body().len())
                }
                
            }
        );
        match request.send_async().await {
            Ok(_) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    /**
     * Attempt to parse a string into an envelope
     */
    pub fn try_new_from_body(body: String, fullBody: Vec<u8>, is_safe: bool) -> Result<SentryEnvelope, AError> {
        // if body.lines().count() == 3 {
        //     let header = body.lines().next().ok_or(BodyError::InvalidNumberOfLines)?;
        //     let header: Value =
        //         serde_json::from_str(header).map_err(|e| BodyError::InvalidHeaderJson(e))?;
        //     if let Some(dsn) = header.get("dsn") {
        //         if let Some(dsn_str) = dsn.as_str() {
        //             let dsn = Dsn::from_str(dsn_str)?;
        //             Ok(SentryEnvelope {
        //                 dsn,
        //                 raw_body: fullBody,
        //             })
        //         } else {
        //             Err(AError::new(BodyError::InvalidDsnValue))
        //         }
        //     } else {
        //         Err(AError::new(BodyError::MissingDsnKeyInHeader))
        //     }
        // } else {
        //     Err(AError::new(BodyError::InvalidNumberOfLines))
        // }

        // Parse line by line until we find the dsn (Max 10 lines)
        let mut dsn: Option<Dsn> = None;
        let mut lines = body.lines();
        for _ in 0..50 {
            let line = lines.next().ok_or(BodyError::InvalidNumberOfLines)?;
            let header: Value = serde_json::from_str(line).map_err(|e| BodyError::InvalidHeaderJson(e))?;
            if let Some(dsn_value) = header.get("dsn") {
                if let Some(dsn_str) = dsn_value.as_str() {
                    dsn = Some(Dsn::from_str(dsn_str)?);
                    break;
                } else {
                    return Err(AError::new(BodyError::InvalidDsnValue));
                }
            }
        }
        // SentryEnvelope
        if let Some(dsn) = dsn {
            Ok(SentryEnvelope {
                dsn,
                raw_body: fullBody,
                is_safe,
            })
        } else {
            Err(AError::new(BodyError::MissingDsnKeyInHeader))
        }
    }
}
