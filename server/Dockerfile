# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2026 Theo-lbg
#
# Drivich API server. Build context: the repository root.
#   docker build -f server/Dockerfile -t drivich-server .

FROM rust:1-slim-bookworm@sha256:a24e292b4af7bdee351a31d3a6379972a5652f0ea22a1b6d6883324ac4f5097c AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN --mount=type=cache,id=drivich-cargo-registry,sharing=locked,target=/usr/local/cargo/registry \
    --mount=type=cache,id=drivich-server-target,target=/src/target \
    cargo build --release --locked -p drivich-server \
 && cp target/release/drivich-server /drivich-server

FROM debian:bookworm-slim@sha256:7ff6ba31527b9e336162fa061746c366c4b2032f4d2cff2078bea43f78efe3f8
RUN apt-get update \
 && apt-get upgrade -y \
 && apt-get install -y --no-install-recommends ca-certificates tini \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --system --uid 10001 --home-dir /data --shell /usr/sbin/nologin drivich \
 && mkdir -p /data && chown drivich:drivich /data
COPY --from=build /drivich-server /usr/local/bin/drivich-server
USER drivich
ENV DRIVICH_BIND=0.0.0.0:2283 \
    DRIVICH_DATA_DIR=/data \
    RUST_LOG=info
VOLUME ["/data"]
EXPOSE 2283
HEALTHCHECK --interval=15s --timeout=5s --start-period=20s --retries=3 \
    CMD ["drivich-server", "--healthcheck"]
ENTRYPOINT ["tini", "--", "drivich-server"]
