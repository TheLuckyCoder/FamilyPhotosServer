FROM rust:1.72.1-bookworm as builder

RUN apt-get update && \
    apt-get install -y \
    musl-tools

RUN rustup target add x86_64-unknown-linux-musl

# create a new empty shell project
RUN USER=root cargo new --bin familyphotos
WORKDIR /familyphotos

# copy manifests
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

# cache your dependencies
RUN cargo build --release --target x86_64-unknown-linux-musl
RUN rm src/*.rs

# copy everything else
COPY . .

RUN rm ./target/x86_64-unknown-linux-musl/release/deps/familyphotos*
RUN cargo build --release --target x86_64-unknown-linux-musl

FROM alpine:3.18

RUN apk add --no-cache heif-thumbnailer ffmpegthumbnailer

COPY --from=builder /familyphotos/target/x86_64-unknown-linux-musl/release/familyphotos .

ENTRYPOINT ["./familyphotos"]