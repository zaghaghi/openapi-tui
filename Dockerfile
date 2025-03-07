# syntax=docker/dockerfile:1

ARG RUST_VERSION=1.84.0

FROM rust:${RUST_VERSION}-slim-bullseye AS build

WORKDIR /app

RUN apt-get update && apt-get install -y build-essential
RUN --mount=type=bind,source=src,target=src \
    --mount=type=bind,source=.config,target=.config \
    --mount=type=bind,source=Cargo.toml,target=Cargo.toml \
    --mount=type=bind,source=Cargo.lock,target=Cargo.lock \
    --mount=type=bind,source=build.rs,target=build.rs \
    --mount=type=cache,target=/app/target/ \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
    <<EOF
set -e
cargo build --locked --release
cp ./target/release/openapi-tui /bin/openapi-tui
strip /bin/openapi-tui
chmod +x /bin/openapi-tui
EOF


FROM debian:bullseye-slim AS final

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates

RUN update-ca-certificates

COPY --from=build /bin/openapi-tui /bin/

ENTRYPOINT [ "/bin/openapi-tui"]
