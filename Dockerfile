FROM rust:1-slim-bullseye AS builder
WORKDIR /app

COPY Cargo.toml .
COPY src/ ./src
RUN mkdir /build

# Use Docker's caching mechanism to improve subsequent builds
# Build the source code and copy the output binary to the /build directory
RUN --mount=id=honeyprint,type=cache,target=/usr/local/cargo/registry \
    --mount=id=honeyprint,type=cache,target=/app/target \
    cargo build --release &&  cp /app/target/release/honeyprint /build/

FROM debian:bullseye-slim

# Install ghostscript and clean up apt cache
RUN apt-get update && apt-get install -y ghostscript && apt-get clean && rm -rf /var/lib/apt/lists/*

# Create a user `debian` with a home directory and bash as the default shell
RUN useradd -ms /bin/bash debian

# Switch to the new user
USER debian

# Set the user's home directory as the working directory
WORKDIR /home/debian

COPY --from=builder /build/honeyprint .

# Start bash when the container is run
ENTRYPOINT ["/home/debian/honeyprint"]