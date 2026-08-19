FROM rust:1.88-bookworm AS builder
WORKDIR /src
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && printf 'fn main() {}\n' > src/main.rs
RUN cargo build --release
COPY src ./src
RUN touch src/main.rs && cargo build --release

FROM debian:bookworm-slim
RUN useradd --system --uid 10001 --create-home app
RUN install -d -o app -g app /data
COPY --from=builder /src/target/release/bmlrssbot /usr/local/bin/bmlrssbot
USER app
WORKDIR /data
ENV RUST_LOG=info STATE_FILE=/data/state.json
ENTRYPOINT ["/usr/local/bin/bmlrssbot"]
