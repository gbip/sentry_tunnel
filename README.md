# Sentry Tunnel

This is a proxy that forwards tunneled sentry requests to the real sentry server.
The implementation is based on the explanation provided by [the official sentry documentation](https://docs.sentry.io/platforms/javascript/troubleshooting/#using-the-tunnel-option).

This proxy looks for the following environnement variables : 

* `TUNNEL_REMOTE_HOST` : The url where to forward the tunneled requests. Example : `TUNNEL_REMOTE_HOST=https://sentry.example.com`.
* `TUNNEL_PROJECT_IDS` : A comma separated list of valid project ids. Request that are not from those projects will be rejected. Example : `TUNNEL_PROJECT_IDS=456,78,10840`.
* `TUNNEL_LISTEN_PORT` : The port that this application will bind to. Example : `TUNNEL_LISTEN_PORT=7878`. This is optional, the default value is 7878.
* `TUNNEL_PATH` : The url path where the tunnel will be waiting for tunneled request. Example : `TUNNEL_PATH=/tunnel`. This is optional, the default value is '/tunnel'.



## Using in a docker stack

## License
