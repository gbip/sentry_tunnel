####################################################################################################
## Builder
####################################################################################################
FROM rust:1.75-alpine AS builder

RUN apk add --no-cache \
    make \
    musl-dev \
    openssl-dev \
    perl \
    pkgconfig

# Create appuser
ENV USER=sentry_tunnel
ENV UID=10001

RUN addgroup -g "${UID}" -S "${USER}" \
 && adduser -h /nonexistent -s /sbin/nologin -G "${USER}" -S -u "${UID}" "${USER}"

# Create an empty application to cache dependency build
RUN cargo new /sentry_tunnel --bin 
WORKDIR /sentry_tunnel
RUN touch src/lib.rs
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
RUN cargo build --release

# Dependency have been built, remove the empty app and copy real source
RUN rm src/*.rs
COPY ./src ./src
RUN touch src/main.rs
RUN touch src/lib.rs
RUN rm -f ./target/release/deps/sentry_tunnel*
RUN rm -f ./target/release/deps/libsentry_tunnel*
RUN cargo build --release

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
COPY --from=builder /sentry_tunnel/target/release/sentry_tunnel ./
# Use an unprivileged user.
USER sentry_tunnel:sentry_tunnel

CMD ["/sentry_tunnel/sentry_tunnel"]
