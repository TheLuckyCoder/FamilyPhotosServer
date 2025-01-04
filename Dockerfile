ARG TARGET_ARCH=x86_64-unknown-linux-musl

FROM rust:1.83-alpine AS base
USER root

RUN apk add --no-cache musl-dev

ARG TARGET_ARCH
RUN rustup target add $TARGET_ARCH

RUN cargo install cargo-chef
WORKDIR /familyphotos


FROM base AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json


FROM base AS builder
COPY --from=planner /familyphotos/recipe.json recipe.json
RUN cargo chef cook --release --target $TARGET_ARCH --recipe-path recipe.json
COPY . .
RUN cargo build --release --target $TARGET_ARCH

FROM alpine:3.21

ARG TARGET_ARCH

RUN apk add --no-cache imagemagick imagemagick-heic ffmpegthumbnailer curl

COPY --from=builder /familyphotos/target/${TARGET_ARCH}/release/familyphotos ./

ENTRYPOINT ["./familyphotos"]