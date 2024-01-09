####################################################################################################
## Builder
####################################################################################################
FROM rust:1.75 AS builder

ARG ARCH=x86_64

RUN rustup target add ${ARCH}-unknown-linux-musl
RUN apt update && apt install -y musl-tools musl-dev libssl-dev pkg-config curl g++
RUN update-ca-certificates

# Create appuser
ENV USER=sentry_tunnel
ENV UID=10001

RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    "${USER}"

# Create an empty application to cache dependency build
RUN cargo new /sentry_tunnel --bin 
WORKDIR /sentry_tunnel
RUN touch src/lib.rs
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
RUN cargo build --target ${ARCH}-unknown-linux-musl --release

# Dependency have been built, remove the empty app and copy real source
RUN rm src/*.rs
COPY ./src ./src
RUN touch src/main.rs
RUN touch src/lib.rs
RUN rm -f ./target/release/deps/sentry_tunnel*
RUN rm -f ./target/release/deps/libsentry_tunnel*
RUN cargo build --target ${ARCH}-unknown-linux-musl --release
RUN mkdir /release/

RUN cp ./target/${ARCH}-unknown-linux-musl/release/sentry_tunnel /release/sentry_tunnel

#===========================#
# Install ssl certificates  #
#===========================#
FROM alpine:3.14 as alpine
RUN apk add -U --no-cache ca-certificates

# ==========================#
# Final image				#
# ==========================#
FROM scratch

# Import from builder.
COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/

WORKDIR /sentry_tunnel

# Copy our build
COPY --from=builder /release/sentry_tunnel ./
COPY --from=alpine /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
# Use an unprivileged user.
USER sentry_tunnel:sentry_tunnel

CMD ["/sentry_tunnel/sentry_tunnel"]
