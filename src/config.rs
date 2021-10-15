use std::fmt::{Display, Formatter};

use envmnt::ListOptions;

#[derive(Debug)]
pub struct Config {
    pub remote_host: String,
    pub project_ids: Vec<String>,
    pub port: u16,
    pub tunnel_path: String,
}

impl Display for Config {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.write_fmt(format_args!("Listening on port {}{}\nForwarding requests to {}.\nValid project ids : {:?}", self.port, self.tunnel_path, self.remote_host, self.project_ids))
    }
}

impl Config {
    /**
     * Create a new config from env variables :
     * - TUNNEL_REMOTE_HOST : String with the distant sentry host
     * - TUNNEL_PROJECT_IDS : Comma separated list of valid project ids that can be forwarded to
     * sentry
     * - TUNNEL_LISTEN_PORT : Optionnal listen port, 7878 by default
     * - TUNNEL_PATH : Url path where this tunnel is waiting for sentry requests. By default
     * "/tunnel"
     */
    pub fn new_from_env_variables() -> Result<Config, String> {
        let mut options = ListOptions::new();
        options.separator = Some(",".to_string());
        let remote_host : String = envmnt::get_parse("TUNNEL_REMOTE_HOST").map_err(|_| "Missing sentry remote. Please the environnement variable 'TUNNEL_REMOTE_HOST' to specify the sentry remote.".to_string())?;
        let project_ids = envmnt::get_list_with_options("TUNNEL_PROJECT_IDS", &options)
            .ok_or(
            "Project ID unspecified. Use 'export TUNNEL_PROJECT_IDS' to provide valid ids."
                .to_string(),
        )?;
        let port = envmnt::get_u16("TUNNEL_LISTEN_PORT", 7878);
        let tunnel_path: String = envmnt::get_parse("TUNNEL_PATH").unwrap_or("/tunnel".to_string());
        Ok(Config {
            remote_host,
            project_ids,
            port,
            tunnel_path,
        })
    }
}
