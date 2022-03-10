# syntax=docker/dockerfile:1.3

ARG RUST_VERSION=1.59
FROM node:lts-alpine AS yarn-builder
ENV YARN_CACHE_FOLDER=/opt/yarncache

WORKDIR /app/grafana-materialize-datasource

# Install yarn dependencies.
COPY ./package.json ./yarn.lock /app/grafana-materialize-datasource/
RUN --mount=type=cache,target=/opt/yarncache yarn install --frozen-lockfile

# Build plugin frontend.
COPY ./README.md ./CHANGELOG.md ./LICENSE ./jest.config.js ./.prettierrc.js ./tsconfig.json /app/grafana-materialize-datasource/
COPY src /app/grafana-materialize-datasource/src
RUN yarn build

FROM rust:${RUST_VERSION}-alpine AS rust-builder

RUN apk add --no-cache musl-dev protoc && \
  rustup component add rustfmt

WORKDIR /usr/src/backend

COPY ./backend /usr/src/backend

RUN \
  --mount=type=cache,id=grafana-materialize-datasource-target-build-cache,target=/usr/src/backend/target/release/build \
  --mount=type=cache,id=grafana-materialize-datasource-target-build-deps,target=/usr/src/backend/target/release/deps \
  --mount=type=cache,id=grafana-materialize-datasource-target-build-incremental,target=/usr/src/backend/target/release/incremental \
  --mount=type=cache,id=grafana-materialize-datasource-cargo-git-cache,target=/usr/local/cargo/git \
  --mount=type=cache,id=grafana-materialize-datasource-cargo-registry-cache,target=/usr/local/cargo/registry \
  cargo build --release

FROM grafana/grafana:8.4.3

# Used to get the target plugin binary name.
ARG TARGETPLATFORM

# Copy plugin files into custom location, to avoid conflicting with contents of /var/lib/grafana. Point
# Grafana to this directory as additional plugin path with the GF_PATHS_PLUGINS env var.
ENV GF_DEFAULT_APP_MODE development
ENV GF_PATHS_PLUGINS /home/grafana/plugins
RUN mkdir -p ${GF_PATHS_PLUGINS }
COPY --chown=grafana --from=yarn-builder /app/grafana-materialize-datasource/dist ${GF_PATHS_PLUGINS }/grafana-materialize-datasource/dist
COPY --chown=grafana --from=rust-builder /usr/src/backend/target/release/grafana-materialize-datasource ${GF_PATHS_PLUGINS }/grafana-materialize-datasource/dist/gpx_grafana-materialize-datasource
RUN GOARCH=$(echo ${TARGETPLATFORM} | sed 's|/|_|') \
  && mv ${GF_PATHS_PLUGINS }/grafana-materialize-datasource/dist/gpx_grafana-materialize-datasource ${GF_PATHS_PLUGINS }/grafana-materialize-datasource/dist/gpx_grafana-materialize-datasource_${GOARCH}

