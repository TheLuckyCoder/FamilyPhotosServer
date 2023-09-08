FROM rust:1.72.0-bookworm as builder

# create a new empty shell project
RUN USER=root cargo new --bin familyphotos
WORKDIR /familyphotos

# copy over your manifests
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

# this build step will cache your dependencies
RUN cargo build --release
RUN rm src/*.rs

# copy your source tree
COPY . .

# build for release
RUN rm ./target/release/deps/familyphotos*
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
	heif-thumbnailer ffmpegthumbnailer && \
	rm -rf /var/lib/apt/lists/*

COPY --from=builder /familyphotos/target/release/familyphotos .

ENTRYPOINT ["./familyphotos"]
