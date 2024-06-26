# Use an official Rust image as the base image
FROM rust:1.74-slim as builder

# Install protobuf
RUN apt-get update && apt-get install -y protobuf-compiler libssl-dev pkg-config

# Create a new empty shell project
RUN USER=root cargo new --bin da
WORKDIR /da

# Copy over your manifests and other necessary files
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
COPY ./build.rs ./build.rs
COPY ./disperser.proto ./disperser.proto
COPY ./src ./src

# Build your application
RUN cargo build --release

# Runtime stage
FROM rust:1.74-slim
RUN apt-get update && apt-get install -y protobuf-compiler openssl libssl-dev && rm -rf /var/lib/apt/lists/*

# Copy the binary from the builder stage
COPY --from=builder /da/target/release/da /usr/local/bin/da

COPY ./static ./static

# Set the startup command to run your binary
CMD ["da"]
