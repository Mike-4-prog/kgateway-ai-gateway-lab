# Stage 1: Build the Rust module
FROM rust:1.88-bookworm AS builder
WORKDIR /build

# Install clang (needed for Envoy SDK C bindings)
RUN apt-get update && apt-get install -y clang

# Copy Rust source code from internal/envoyinit structure
COPY internal/envoyinit/ ./internal/envoyinit/

# Build the Rust module in release mode
WORKDIR /build/internal/envoyinit/rustformations
RUN cargo build --release

# Stage 2: Final Envoy image - UPDATED to 1.37.2 for v2.3 compatibility
FROM envoyproxy/envoy:v1.36.4

# Copy the compiled Rust module
COPY --from=builder /build/internal/envoyinit/rustformations/target/release/librust_module.so /usr/local/lib/

# Tell Envoy where to find dynamic modules
ENV ENVOY_DYNAMIC_MODULES_SEARCH_PATH=/usr/local/lib

# Copy Envoy configuration
COPY envoy.yaml /etc/envoy/envoy.yaml

# Run Envoy
CMD ["envoy", "-c", "/etc/envoy/envoy.yaml"]