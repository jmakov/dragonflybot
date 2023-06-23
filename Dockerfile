ARG BUILD_PROFILE="performance"

FROM clearlinux/os-core:latest as clearlinux-core

FROM clearlinux:latest AS base
# install system dependencies needed to build
RUN swupd update -y --no-boot-update --3rd-party  \
    && swupd bundle-add rust-basic devpkg-openssl openssl protobuf \
    && cargo install cargo-chef
WORKDIR app

FROM base AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM base AS builder
ARG BUILD_PROFILE
ENV RUSTFLAGS="-C target-cpu=native"
COPY --from=planner /app/recipe.json recipe.json
# build dependencies - this is the caching Docker layer
RUN cargo chef cook --release --recipe-path recipe.json
# build application
COPY . .
RUN cargo build --profile $BUILD_PROFILE

# branch build targets
# `clearlinux-core` is updated daily, so we don't need to run updates
FROM clearlinux-core AS branch-performance
COPY --from=builder /app/target/release/dragonflybot-* /usr/local/bin/
FROM clearlinux-core AS branch-dev
COPY --from=builder /app/target/debug/dragonflybot-* /usr/local/bin/
FROM branch-${BUILD_PROFILE} as after-condition

# minimize the image from a couple GB to about 100MB
FROM after-condition AS dragonflybot
WORKDIR /usr/local/bin
CMD ["./dragonflybot-grpc-server"]