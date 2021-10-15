use gotham::handler::HandlerError;
use gotham::hyper::StatusCode;

use anyhow::Error as AError;

use isahc::{Request, RequestExt};

use serde_json::Value;

use log::*;

use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

#[derive(Debug)]
pub struct RemoteSentryInstance {
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
        }
    }
}

impl Error for BodyError {}

pub fn make_error<T>(err: T) -> HandlerError
where
    T: Into<AError>,
{
    HandlerError::from(err.into()).with_status(StatusCode::BAD_REQUEST)
}

impl RemoteSentryInstance {
    pub async fn forward(self, host: &str) -> Result<(), AError> {
        let request = Request::builder()
            .uri(host)
            .header("Content-type", "application/x-sentry-envelope")
            .method("POST")
            .body(self.raw_body)?;
        info!(
            "{} {} - body={}",
            request.method(),
            request.uri(),
            request.body()
        );
        request.send_async().await?;
        Ok(())
    }

    pub fn try_new_from_body(body: String) -> Result<RemoteSentryInstance, HandlerError> {
        if body.lines().count() == 3 {
            let header = body
                .lines()
                .next()
                .ok_or_else(|| make_error(BodyError::MalformedBody))?;
            let header: Value = serde_json::from_str(header).map_err(|e| {
                make_error(BodyError::InvalidHeaderJson(e)).with_status(StatusCode::BAD_REQUEST)
            })?;
            if let Some(dsn) = header.get("dsn") {
                if let Some(dsn_str) = dsn.as_str() {
                    let (_url, project_id) = dsn_str
                        .rsplit_once('/')
                        .ok_or_else(|| make_error(BodyError::MalformedBody))?;
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