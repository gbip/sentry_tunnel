use gotham::anyhow::Error as AError;
use gotham::handler::IntoResponse;
use gotham::helpers::http::response::create_response;
use gotham::hyper::StatusCode;
use gotham::hyper::{body::Body, Response};
use gotham::state::State;
use isahc::{Request, RequestExt};
use mime::Mime;
use serde_json::Value;

use log::*;

use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

#[derive(Debug)]
pub struct RemoteSentryInstance {
    pub project_id: String,
    pub raw_body: String,
}

#[derive(Debug)]
pub enum BodyError {
    MalformedBody,
    InvalidHeaderJson(serde_json::Error),
    MissingDsnKeyInHeader,
    InvalidProjectId,
}

impl Display for BodyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BodyError::MalformedBody => f.write_str("Malformed HTTP Body"),
            BodyError::MissingDsnKeyInHeader => f.write_str("dsn key was not found in header"),
            BodyError::InvalidHeaderJson(e) => f.write_fmt(format_args!("{}", e)),
            BodyError::InvalidProjectId => f.write_str("Unauthorized project ID"),
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

impl RemoteSentryInstance {
    pub async fn forward(self, host: &str) -> Result<(), AError> {
        let uri = format!("{}/api/{}/envelope", host, self.project_id);
        let request = Request::builder()
            .uri(uri)
            .header("Content-type", "application/x-sentry-envelope")
            .method("POST")
            .body(self.raw_body)?;
        info!(
            "Sending HTTP {} {} - body={}",
            request.method(),
            request.uri(),
            request.body()
        );
        request.send_async().await?;
        Ok(())
    }

    pub fn try_new_from_body(body: String) -> Result<RemoteSentryInstance, BodyError> {
        if body.lines().count() == 3 {
            let header = body.lines().next().ok_or(BodyError::MalformedBody)?;
            let header: Value =
                serde_json::from_str(header).map_err(|e| BodyError::InvalidHeaderJson(e))?;
            if let Some(dsn) = header.get("dsn") {
                if let Some(dsn_str) = dsn.as_str() {
                    let (_url, project_id) =
                        dsn_str.rsplit_once('/').ok_or(BodyError::MalformedBody)?;
                    Ok(RemoteSentryInstance {
                        project_id: project_id.to_string(),
                        raw_body: body,
                    })
                } else {
                    Err(BodyError::MalformedBody)
                }
            } else {
                Err(BodyError::MissingDsnKeyInHeader)
            }
        } else {
            Err(BodyError::MalformedBody)
        }
    }
}
