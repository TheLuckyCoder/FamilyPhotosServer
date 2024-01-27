ARG TARGET_ARCH=x86_64-unknown-linux-musl

FROM rust:1.75-bookworm as builder

ARG TARGET_ARCH

RUN apt-get update && \
    apt-get install -y \
    musl-tools

RUN rustup target add $TARGET_ARCH

# create a new empty shell project
RUN USER=root cargo new --bin familyphotos
WORKDIR /familyphotos

# copy manifests
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

# cache dependencies
RUN cargo build --release --target ${TARGET_ARCH}
RUN rm src/*.rs

# copy everything else
COPY . .

RUN rm ./target/${TARGET_ARCH}/release/deps/familyphotos*
RUN cargo build --release --target ${TARGET_ARCH}

FROM alpine:3.19

ARG TARGET_ARCH

RUN apk add --no-cache imagemagick imagemagick-heic ffmpegthumbnailer curl

COPY --from=builder /familyphotos/target/${TARGET_ARCH}/release/familyphotos .

ENTRYPOINT ["./familyphotos"]