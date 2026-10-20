# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2026 Theo-lbg
#
# Drivich web: the SvelteKit app is built to static files and served by nginx,
# which also proxies /api to the server (streaming, no body buffering, so very
# large uploads and downloads pass through untouched).
# Build context: the web/ directory.
#   docker build -f web/Dockerfile -t drivich-web web

# The static files do not depend on the CPU, so they are always built on the build
# machine's own platform (no emulation, which crashes Node under QEMU).
FROM --platform=$BUILDPLATFORM node:26-alpine@sha256:6f08a523443ddedf52941874c203475b0ad503e859ad70cb26ab8be9a517dbd5 AS build
WORKDIR /app
COPY package.json package-lock.json .npmrc ./
RUN npm ci
COPY . .
RUN npm run build

FROM nginxinc/nginx-unprivileged:stable-alpine@sha256:f5116ed4606f3eb015dd4a8e28cb9b351476dc0ad4bc6e9f93b19cec130cdf03
COPY nginx.conf.template /etc/nginx/templates/default.conf.template
COPY --from=build /app/build /usr/share/nginx/html
ENV DRIVICH_SERVER_URL=http://server:2283
EXPOSE 8080
HEALTHCHECK --interval=15s --timeout=5s --start-period=10s --retries=3 \
    CMD wget -q -O /dev/null http://127.0.0.1:8080/healthz || exit 1
