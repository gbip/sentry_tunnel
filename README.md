[![tests](https://github.com/gbip/sentry_tunnel/actions/workflows/build_test.yml/badge.svg)](https://github.com/gbip/sentry_tunnel/actions/workflows/build_test.yml)
![Image Size](https://img.shields.io/docker/image-size/sigalen/sentry_tunnel)
![Docker Pulls](https://img.shields.io/docker/pulls/sigalen/sentry_tunnel)

# Sentry Tunnel

This is a proxy that forwards tunneled sentry requests to the real sentry server.
The implementation is based on the explanation provided by [the official sentry documentation](https://docs.sentry.io/platforms/javascript/troubleshooting/#using-the-tunnel-option).

> A tunnel is an HTTP endpoint that acts as a proxy between Sentry and your application. Because you control this server, there is no risk of any requests sent to it being blocked. When the endpoint lives under the same origin (although it does not have to in order for the tunnel to work), the browser will not treat any requests to the endpoint as a third-party request. As a result, these requests will have different security measures applied which, by default, don't trigger ad-blockers.

[From the sentry documentation](https://docs.sentry.io/platforms/javascript/troubleshooting/#using-the-tunnel-option)

Please note that the **minimal supported Relay version is v21.6.0**. Older versions might work, but are not supported by this project. 
[Explanation here](https://develop.sentry.dev/sdk/envelopes/#authentication)

## Configuration

This proxy looks for the following environnement variables : 

* `TUNNEL_REMOTE_HOST` : A comma separted list of sentry relays which are allowed to be tuneled by this service. Example : `TUNNEL_REMOTE_HOST=https://sentry.example.com, https://sentry2.example.com`.
* `TUNNEL_PROJECT_IDS` : A comma separated list of valid project ids. Request that are not from those projects will be rejected. Example : `TUNNEL_PROJECT_IDS=456,78,10840`.
* `TUNNEL_LISTEN_PORT` : The port that this application will bind to. Example : `TUNNEL_LISTEN_PORT=7878`. This is optional, the default value is 7878.
* `TUNNEL_PATH` : The url path where the tunnel will be waiting for tunneled request. Example : `TUNNEL_PATH=/tunnel`. This is optional, the default value is '/tunnel'.
* `TUNNEL_IP` : The ip that this application will listen on. Optional, the default value is `127.0.0.1`.

## Running with docker

The docker image [lives here](https://hub.docker.com/repository/docker/sigalen/sentry_tunnel).

An example docker-compose file is provided. Otherwise : 

```
docker run --rm -e 'TUNNEL_REMOTE_HOST=https://sentry.example.com' -e 'TUNNEL_PROJECT_IDS=1,5' sigalen/sentry_tunnel
```

## Running without docker

```bash
python3 -m venv venv  # Install venv
. venv/bin/activate  # Enable venv
pip install -r requirements.txt  # Install dependencies
./manage.py collectstatic
./manage.py makemessages -l fr
cp env/.env.docker.prod.djrdo.example .env
vim .env  # Edit env file, you can remove VIRTUAL_HOST and LETSENCRYPT_HOST lines
hypercorn --bind 0.0.0.0:8000 djRDO.asgi:application  # You should put this line in a service file :)
```

Here is an example nginx configuration :

```nginx
server {
	listen *:443 ssl;
	server_name demo.djrdo.florencepaul.com;

	include ssl.conf;

	ssl_certificate fullchain.pem;
	ssl_certificate_key privkey.pem;

	include certbot.conf;
	keepalive_timeout 5;

	location /static/ {
		alias   /<djrdo_path>/static/;
	}
	location / {
		proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
		proxy_set_header Host $http_host;
		proxy_redirect off;
		proxy_connect_timeout 90;
		proxy_send_timeout 180;
		proxy_read_timeout 180;
		proxy_buffer_size 16k;
		proxy_buffers 8 16k;
		proxy_busy_buffers_size 32k;
		proxy_intercept_errors on;
		if (!-f $request_filename) {
			proxy_pass http://djrdo_backend;
			break;
		}
	}
}

upstream djrdo_backend {
	server 127.0.0.1:8000;
}


```

## License

BSD-2
