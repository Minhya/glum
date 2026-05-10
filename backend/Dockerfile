# Stage 1: compile the Rust binary
FROM rust:latest AS builder

# Install protoc for compiling proto files
RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .
RUN cargo build --release

# Stage 2: minimal runtime image
FROM debian:bookworm-slim

# Required for TLS and HTTPS connections
RUN apt-get update && apt-get install -y libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy only the compiled binary from the builder stage
COPY --from=builder /app/target/release/glum-backend .

EXPOSE 3000 3001
CMD ["./glum-backend"]
