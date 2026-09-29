# Backend container image for Tax Document Routing.
#
# The backend is built on the PDS App Framework, which is consumed as a pinned
# sibling checkout (`../app-framework`, see the repository root README and
# appfw.lock). A container build therefore needs BOTH this repository and the
# framework checkout in the build context, so the context is the PARENT
# directory that holds them:
#
#   <parent>/
#   |-- app-framework/        # PDS App Framework, pinned
#   `-- <this repo>/          # default dir name: tax-doc-routing
#
# Build from <parent>/ :
#
#   docker build \
#     -f tax-doc-routing/backend/Dockerfile \
#     --build-arg PRODUCT_DIR=tax-doc-routing \
#     --build-arg FRAMEWORK_DIR=app-framework \
#     -t tax-doc-routing-backend .
#
# In PDS CI the framework is instead resolved from the ProGet Cargo registry
# (`proget.pdsconnect.com`), and this Dockerfile's context can then be the
# product repo alone — drop the FRAMEWORK_DIR copy and rely on
# `.cargo/config.toml` + `appfw.lock`. See docs/architecture/deployment-pds.md.

ARG PRODUCT_DIR=tax-doc-routing
ARG FRAMEWORK_DIR=app-framework
ARG RUST_VERSION=1.98
ARG NODE_VERSION=22
ARG DEBIAN_RELEASE=bookworm

# ---- stage 1: build the product SPA ------------------------------------
FROM node:${NODE_VERSION}-${DEBIAN_RELEASE}-slim AS frontend
ARG PRODUCT_DIR
WORKDIR /src/${PRODUCT_DIR}/frontend
COPY ${PRODUCT_DIR}/frontend/package.json ${PRODUCT_DIR}/frontend/package-lock.json* ./
RUN npm ci
COPY ${PRODUCT_DIR}/frontend/ ./
# vite.config.ts emits the bundle to ../backend/product_dist
RUN npm run build

# ---- stage 2: build the backend binary --------------------------------
FROM rust:${RUST_VERSION}-${DEBIAN_RELEASE} AS builder
ARG PRODUCT_DIR
ARG FRAMEWORK_DIR
WORKDIR /src
# The framework checkout first: backend/Cargo.toml path-depends on
# ../<FRAMEWORK_DIR>/appfw_runtime etc. relative to the product repo.
COPY ${FRAMEWORK_DIR}/ ./${FRAMEWORK_DIR}/
COPY ${PRODUCT_DIR}/ ./${PRODUCT_DIR}/
WORKDIR /src/${PRODUCT_DIR}
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/${PRODUCT_DIR}/target \
    cargo build --release -p backend --bin backend \
 && cp target/release/backend /usr/local/bin/backend

# ---- stage 3: runtime -------------------------------------------------
FROM debian:${DEBIAN_RELEASE}-slim AS runtime
ARG PRODUCT_DIR
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates curl \
 && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /usr/local/bin/backend /usr/local/bin/backend
# The backend reads generated config from ${CWD}/config/generated/** (see
# backend/src/config/loader.rs) and serves the SPA from APP_PRODUCT_UI_DIST_DIR.
COPY --from=builder /src/${PRODUCT_DIR}/backend/config ./config
COPY --from=frontend /src/${PRODUCT_DIR}/backend/product_dist ./product_dist
ENV APP_PRODUCT_UI_DIST_DIR=/app/product_dist \
    API_HOST=0.0.0.0 \
    API_PORT=8080
EXPOSE 8080
# Worker-thread stack size is BACKEND_WORKER_STACK_MIB (default 128 MiB, see
# backend/src/main.rs) — no RUST_MIN_STACK needed.
ENTRYPOINT ["/usr/local/bin/backend"]
