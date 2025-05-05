FROM rust:1.86.0 as builder

WORKDIR /usr/src/app

RUN rustup target add x86_64-unknown-linux-musl

COPY Cargo.toml Cargo.lock ./
COPY src src
RUN cargo build --target x86_64-unknown-linux-musl --release

FROM rust:alpine3.17

COPY --from=builder /usr/src/app/target/release/mcp /usr/local/bin/mcp

CMD ["mcp"]