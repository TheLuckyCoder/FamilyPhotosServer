FROM rust:bookworm as builder

RUN apt-get update
RUN apt-get install postgresql libwebp-dev libjemalloc-dev -y --no-install-recommends

COPY . .

RUN cargo build --release

FROM debian:bookworm-slim

COPY --from=builder /target/release/familyphotos .

RUN apt-get update && \
	apt-get install postgresql heif-thumbnailer ffmpegthumbnailer -y --no-install-recommends


CMD ["/familyphotos"]
