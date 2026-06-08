FROM rust:latest

COPY src/* ./src/
COPY Cargo.lock .
COPY Cargo.toml .
COPY rust-toolchain.toml .

RUN cargo install --locked --path .

ENTRYPOINT ["unifi-protect-bulk-download"]
