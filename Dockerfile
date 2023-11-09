ARG TARGET_ARCH=x86_64-unknown-linux-musl

FROM rust:1.73-bookworm as builder

RUN apt-get update && \
    apt-get install -y \
    musl-tools

RUN rustup target add ${TARGET_ARCH_TRIPLE}

# create a new empty shell project
RUN USER=root cargo new --bin familyphotos
WORKDIR /familyphotos

# copy manifests
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

# cache dependencies
RUN cargo build --release --target ${TARGET_ARCH_TRIPLE}
RUN rm src/*.rs

# copy everything else
COPY . .

RUN rm ./target/${TARGET_ARCH_TRIPLE}/release/deps/familyphotos*
RUN cargo build --release --target ${TARGET_ARCH_TRIPLE}

FROM alpine:3.18

RUN apk add --no-cache heif-thumbnailer ffmpegthumbnailer

COPY --from=builder /familyphotos/target/${TARGET_ARCH_TRIPLE}/release/familyphotos .

ENTRYPOINT ["./familyphotos"]