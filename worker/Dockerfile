# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2026 Theo-lbg
#
# Drivich background worker (thumbnails, checksums, indexing, housekeeping).
# Build context: the repository root.
#   docker build -f worker/Dockerfile -t drivich-worker .
# Video thumbnails need ffmpeg; build with --build-arg WITH_FFMPEG=false for a
# smaller image if you do not need them. The image also contains the PostgreSQL 17
# client (pg_dump) used for the automatic database backups. Office documents are
# rendered to PDF for the viewer with LibreOffice (headless, one at a time, no network
# in the default compose files); build with --build-arg WITH_OFFICE=false to leave it
# out (about 400 MB smaller; Office files then show a download button instead).

FROM rust:1-slim-bookworm@sha256:86234e857a413004348b471d11a4ebe1aa92d2eae2b6b143cac68fd1ac3c48be AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN --mount=type=cache,id=drivich-cargo-registry,sharing=locked,target=/usr/local/cargo/registry \
    --mount=type=cache,id=drivich-worker-target,target=/src/target \
    cargo build --release --locked -p drivich-worker \
 && cp target/release/drivich-worker /drivich-worker

FROM debian:bookworm-slim@sha256:7c7b2c966bc9ee8cedfeef67e0e279108992c77681fa595db4a9d65c06ccc587
ARG WITH_FFMPEG=true
ARG WITH_OFFICE=true
# pg_dump must be at least as recent as the server (17): Debian bookworm only ships 15,
# so the PostgreSQL project's own apt repository is added (key and setup script come from
# the signed `postgresql-common` package, nothing is downloaded with curl).
RUN apt-get update \
 && apt-get upgrade -y \
 && apt-get install -y --no-install-recommends ca-certificates tini postgresql-common \
 && /usr/share/postgresql-common/pgdg/apt.postgresql.org.sh -y \
 && apt-get install -y --no-install-recommends postgresql-client-17 \
 && if [ "$WITH_FFMPEG" = "true" ]; then apt-get install -y --no-install-recommends ffmpeg; fi \
 && if [ "$WITH_OFFICE" = "true" ]; then apt-get install -y --no-install-recommends libreoffice-writer-nogui libreoffice-calc-nogui libreoffice-impress-nogui fonts-liberation fonts-dejavu-core; fi \
 && apt-get purge -y postgresql-common \
 && apt-get autoremove -y \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --system --uid 10001 --home-dir /data --shell /usr/sbin/nologin drivich \
 && mkdir -p /data && chown drivich:drivich /data
COPY --from=build /drivich-worker /usr/local/bin/drivich-worker
USER drivich
ENV DRIVICH_DATA_DIR=/data \
    RUST_LOG=info
VOLUME ["/data"]
ENTRYPOINT ["tini", "--", "drivich-worker"]
