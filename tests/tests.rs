#[cfg(test)]
mod tests {
    use gotham::hyper::http::{header, HeaderValue, StatusCode};
    use gotham::test::TestServer;

    use httpmock::prelude::*;
    use mime::Mime;
    use sentry_tunnel::config::Config;
    use sentry_tunnel::envelope::BodyError;
    use sentry_tunnel::server::{router, HeaderError};

    #[test]
    fn test_correct_behaviour() {
        let server = MockServer::start();
        let sentry_mock = server.mock(|when, then| {
            when.method(POST).path("/api/5/envelope/");
            then.status(200);
        });
        let test_config = Config {
            remote_hosts: vec![server.url("")],
            project_ids: vec!["5".to_string()],
            port: 7878,
            tunnel_path: "/tunnel".to_string(),
            ip: "0.0.0.0".to_string(),
        };
        let test_server = TestServer::new(router(
            &test_config.tunnel_path.clone(),
            test_config.clone(),
        ))
        .unwrap();
        let json = r#"{"sent_at":"2021-10-14T17:10:40.136Z","sdk":{"name":"sentry.javascript.browser","version":"6.13.3"},"dsn":"http://public@HOST_TEST_REPLACE/5"}
        {"type":"session"}
        {"sid":"751d80dc94e34cd282a2cf1fe698a8d2","init":true,"started":"2021-10-14T17:10:40.135Z","timestamp":"2021-10-14T17:10:40.135Z","status":"ok","errors":0,"attrs":{"release":"test_project@1.0"}"#;
        let json = json
            .replace("HOST_TEST_REPLACE", &server.address().to_string())
            .to_owned();
        println!("{:?}", json);
        let mime = "application/json".parse::<Mime>().unwrap();
        let response = test_server
            .client()
            .post(
                "http://localhost".to_owned() + &test_config.tunnel_path,
                json.clone(),
                mime,
            )
            .with_header(
                header::CONTENT_LENGTH,
                HeaderValue::from_str(&format!("{}", json.as_bytes().len())).unwrap(),
            )
            .perform()
            .unwrap();

        sentry_mock.assert();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_invalid_project_id() {
        let test_config = Config {
            remote_hosts: vec!["https://sentry.example.com/".to_string()],
            project_ids: vec!["5".to_string()],
            port: 7878,
            tunnel_path: "/tunnel".to_string(),
            ip: "0.0.0.0".to_string(),
        };
        let test_server = TestServer::new(router(
            &test_config.tunnel_path.clone(),
            test_config.clone(),
        ))
        .unwrap();
        let json = r#"{"sent_at":"2021-10-14T17:10:40.136Z","sdk":{"name":"sentry.javascript.browser","version":"6.13.3"},"dsn":"https://public@sentry.example.com/4"}
        {"type":"session"}
        {"sid":"751d80dc94e34cd282a2cf1fe698a8d2","init":true,"started":"2021-10-14T17:10:40.135Z","timestamp":"2021-10-14T17:10:40.135Z","status":"ok","errors":0,"attrs":{"release":"test_project@1.0"}"#;
        let mime = "application/json".parse::<Mime>().unwrap();
        let response = test_server
            .client()
            .post(
                "http://localhost".to_owned() + &test_config.tunnel_path,
                json,
                mime,
            )
            .with_header(
                header::CONTENT_LENGTH,
                HeaderValue::from_str(&format!("{}", json.as_bytes().len())).unwrap(),
            )
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = response.read_body().unwrap();
        let expc = format!("{}", BodyError::InvalidProjectId);

        assert_eq!(String::from_utf8(body).unwrap(), expc);
    }

    #[test]
    fn test_missing_dsn() {
        let test_config = Config {
            remote_hosts: vec!["https://sentry.example.com/".to_string()],
            project_ids: vec!["5".to_string()],
            port: 7878,
            tunnel_path: "/tunnel".to_string(),
            ip: "0.0.0.0".to_string(),
        };
        let test_server = TestServer::new(router(
            &test_config.tunnel_path.clone(),
            test_config.clone(),
        ))
        .unwrap();
        let json = r#"{"sent_at":"2021-10-14T17:10:40.136Z","sdk":{"name":"sentry.javascript.browser","version":"6.13.3"}}
        {"type":"session"}
        {"sid":"751d80dc94e34cd282a2cf1fe698a8d2","init":true,"started":"2021-10-14T17:10:40.135Z","timestamp":"2021-10-14T17:10:40.135Z","status":"ok","errors":0,"attrs":{"release":"test_project@1.0"}"#;
        let mime = "application/json".parse::<Mime>().unwrap();
        let response = test_server
            .client()
            .post(
                "http://localhost".to_owned() + &test_config.tunnel_path,
                json,
                mime,
            )
            .with_header(
                header::CONTENT_LENGTH,
                HeaderValue::from_str(&format!("{}", json.as_bytes().len())).unwrap(),
            )
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = response.read_body().unwrap();
        let expc = format!("{}", BodyError::MissingDsnKeyInHeader);

        assert_eq!(String::from_utf8(body).unwrap(), expc);
    }

    #[test]
    fn test_dsn_host_invalid() {
        let test_config = Config {
            remote_hosts: vec!["https://sentry.example.com/".to_string()],
            project_ids: vec!["5".to_string()],
            port: 7878,
            tunnel_path: "/tunnel".to_string(),
            ip: "0.0.0.0".to_string(),
        };
        let test_server = TestServer::new(router(
            &test_config.tunnel_path.clone(),
            test_config.clone(),
        ))
        .unwrap();
        let json = r#"{"sent_at":"2021-10-14T17:10:40.136Z","sdk":{"name":"sentry.javascript.browser","version":"6.13.3"},"dsn":"https://public@not_a_valid_host.example.com/5"}
        {"type":"session"}
        {"sid":"751d80dc94e34cd282a2cf1fe698a8d2","init":true,"started":"2021-10-14T17:10:40.135Z","timestamp":"2021-10-14T17:10:40.135Z","status":"ok","errors":0,"attrs":{"release":"test_project@1.0"}"#;
        let mime = "application/json".parse::<Mime>().unwrap();
        let response = test_server
            .client()
            .post(
                "http://localhost".to_owned() + &test_config.tunnel_path,
                json,
                mime,
            )
            .with_header(
                header::CONTENT_LENGTH,
                HeaderValue::from_str(&format!("{}", json.as_bytes().len())).unwrap(),
            )
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = response.read_body().unwrap();
        let expc = format!("{}", HeaderError::InvalidHost);

        assert_eq!(String::from_utf8(body).unwrap(), expc);
    }
}
