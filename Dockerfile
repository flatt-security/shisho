FROM rust:latest AS builder
RUN rustup target add x86_64-unknown-linux-gnu
RUN rustup component add rustfmt
WORKDIR /build

# cache dependencies
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
COPY ./third_party ./third_party
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release

# build
COPY ./ .
RUN cargo build --release

FROM gcr.io/distroless/cc:latest
WORKDIR /workspace
COPY --from=builder /build/target/release/shisho /shisho
ENTRYPOINT ["/shisho"]