# Dev guide

## Publishing a new release

```
docker build --tag sentry_tunnel:latest . # Build docker image
#docker image ls | grep sentry # Find image ID
#docker tag <ID> sigalen/sentry_tunnel:latest # Create image tag (in case of build fail)
docker push sigalen/sentry_tunnel:latest
```

## Building for a different architecture

```
docker build --tag sentry_tunnel:latest --build-arg ARCH=aarch64 .
```
