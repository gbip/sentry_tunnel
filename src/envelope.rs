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
use url::Url;

use log::*;

use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;

/**
 * Represent a sentry envelope
 */
#[derive(Debug)]
pub struct SentryEnvelope {
    pub raw_body: String,
    pub dsn: Dsn,
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
    pub fn dsn_host_is_valid(&self, host: &[String]) -> bool {
        let envelope_host = self.dsn.host().to_string();
        host.iter()
            .map(|h| Url::parse(h).unwrap().host_str().unwrap_or("").to_string())
            .any(|x| x == envelope_host)
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
            .body(self.raw_body.clone())?;
        info!(
            "Sending HTTP {} {} - body={}",
            request.method(),
            request.uri(),
            request.body()
        );
        match request.send_async().await {
            Ok(_) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    /**
     * Attempt to parse a string into an envelope
     */
    pub fn try_new_from_body(body: String) -> Result<SentryEnvelope, AError> {
        if body.lines().count() == 3 {
            let header = body.lines().next().ok_or(BodyError::InvalidNumberOfLines)?;
            let header: Value =
                serde_json::from_str(header).map_err(|e| BodyError::InvalidHeaderJson(e))?;
            if let Some(dsn) = header.get("dsn") {
                if let Some(dsn_str) = dsn.as_str() {
                    let dsn = Dsn::from_str(dsn_str)?;
                    Ok(SentryEnvelope {
                        dsn,
                        raw_body: body,
                    })
                } else {
                    Err(AError::new(BodyError::InvalidDsnValue))
                }
            } else {
                Err(AError::new(BodyError::MissingDsnKeyInHeader))
            }
        } else {
            Err(AError::new(BodyError::InvalidNumberOfLines))
        }
    }
}
