# Use the official Rust image with musl target for static linking
FROM rust:1.90-alpine as builder

# Install musl-dev for static linking
RUN apk add --no-cache musl-dev

# Create a new empty shell project
WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to build dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy the actual source code
COPY src ./src

# Build the application with static linking
# Touch main.rs to force rebuild of the application with the real source
RUN touch src/main.rs && cargo build --release

# Runtime stage - use distroless for minimal size with better debugging
FROM gcr.io/distroless/static-debian12

# Copy the binary from builder
COPY --from=builder /app/target/release/pokedex /pokedex

# Expose port 5000
EXPOSE 5000

# Run the binary
CMD ["/pokedex"]
