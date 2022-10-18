use envmnt::ListOptions;

use std::fmt::{Display, Formatter};
use url::Url;
use log::error;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Host(pub String);

impl Display for Host {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Clone, Debug)]
pub struct Config {
    pub remote_hosts: Vec<Host>,
    pub project_ids: Vec<String>,
    pub port: u16,
    pub tunnel_path: String,
    pub ip: String,
}

impl Display for Config {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "Listening on {}:{}{}\nForwarding requests to : {:?}\nValid project ids : {:?}",
            self.ip, self.port, self.tunnel_path, self.remote_hosts, self.project_ids
        ))
    }
}

impl Config {
    /**
     * Create a new config from env variables :
     * - TUNNEL_REMOTE_HOST : Comma separated list of valid sentry relays
     * - TUNNEL_PROJECT_IDS : Comma separated list of valid project ids that can be forwarded to
     * sentry
     * - TUNNEL_LISTEN_PORT : Optionnal listen port, 7878 by default
     * - TUNNEL_PATH : Url path where this tunnel is waiting for sentry requests. By default
     * - TUNNEL_IP : Listen interface. Optional, 127.0.0.1 by default.
     */
    pub fn new_from_env_variables() -> Result<Config, String> {
        let mut options = ListOptions::new();
        options.separator = Some(",".to_string());
        let remote_hosts  = envmnt::get_list_with_options("TUNNEL_REMOTE_HOST", &options).ok_or_else(|| "Missing sentry remote. Please set the environnement variable 'TUNNEL_REMOTE_HOST' to specify the sentry remote.".to_string())?;
        let project_ids = envmnt::get_list_with_options("TUNNEL_PROJECT_IDS", &options)
            .ok_or_else(|| {
                "Project ID unspecified. Use 'export TUNNEL_PROJECT_IDS' to provide valid ids."
                    .to_string()
            })?;
        let port = envmnt::get_u16("TUNNEL_LISTEN_PORT", 7878);
        let tunnel_path: String =
            envmnt::get_parse("TUNNEL_PATH").unwrap_or_else(|_| "/tunnel".to_string());
        let ip: String = envmnt::get_parse("TUNNEL_IP").unwrap_or_else(|_| "127.0.0.1".to_string());
        let valid_remote_hosts = Config::clean_remote_hosts(&remote_hosts);
        if valid_remote_hosts.len() == 0 {
            Err("No remote hosts to forward sentry envelopes to".to_string())
        } else {
            Ok(Config {
                remote_hosts : valid_remote_hosts,
                project_ids,
                port,
                tunnel_path,
                ip,
            })
        }
    }

    pub fn project_id_is_allowed(&self, id: u64) -> bool {
        let id_str = format!("{}", id);
        self.project_ids.contains(&id_str)
    }

    pub fn clean_remote_hosts(hosts : &[String]) -> Vec<Host>{
        let mut result = vec!();
        for host in hosts {
            if let Ok(host_url) = Url::parse(host) {
                if let Some(hostname) = host_url.host_str() {
                    result.push(Host(hostname.to_string()))
                } else {
                    error!("{} is not an URL to a remote host", host_url)
                }
            }
            else {
                error!("{} is not a valid url", host)
            }
        }
        return result
    }
}
