# Stage 1: Build the Rust module
FROM rust:1.85 AS builder
WORKDIR /build

# Install clang (needed for Envoy SDK C bindings)
RUN apt-get update && apt-get install -y clang

# Copy Rust source code
COPY rust/ ./rust/

# Build the Rust module in release mode
WORKDIR /build/rust/rustformations
RUN cargo build --release

# Stage 2: Final Envoy image
FROM envoyproxy/envoy:v1.36.4

# Copy the compiled Rust module
COPY --from=builder /build/rust/rustformations/target/release/librust_module.so /usr/local/lib/

# Tell Envoy where to find dynamic modules
ENV ENVOY_DYNAMIC_MODULES_SEARCH_PATH=/usr/local/lib

# Copy Envoy configuration
COPY envoy.yaml /etc/envoy/envoy.yaml

# Run Envoy
CMD ["envoy", "-c", "/etc/envoy/envoy.yaml"]
