# kgateway AI Gateway Lab

[![License](https://img.shields.io/badge/License-Apache%202.0-green.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.88+-orange?logo=rust)](https://www.rust-lang.org/)
[![kgateway](https://img.shields.io/badge/kgateway-2.2.x-purple)](https://kgateway.dev/)

Extend kgateway with a custom Rust module that adds `X-Custom-Transformed: true` header to every response.

## Project structure
```text
rust/
├── rustformations/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs              # Registers the filter with Envoy
│       └── http_simple_mutations.rs  # Your actual transformation logic
└── transformations/
    ├── Cargo.toml
    └── src/
        ├── lib.rs              # Defines transformation traits
        └── jinja.rs            # Jinja templating for dynamic transformations
```


## Quick Start

```bash
# Create cluster
kind create cluster --name ai-gateway-lab

# Install kgateway
helm upgrade -i kgateway-crds oci://cr.kgateway.dev/kgateway-dev/charts/kgateway-crds \
  --create-namespace --namespace kgateway-system --version v2.2.1

helm upgrade -i kgateway oci://cr.kgateway.dev/kgateway-dev/charts/kgateway \
  --namespace kgateway-system --version v2.2.1

# Deploy mock LLM
kubectl apply -f httpbun.yaml

# Configure Gateway
kubectl apply -f httpbun-backend.yaml
kubectl apply -f gateway.yaml
kubectl apply -f httpbun-route.yaml

# Build and deploy custom Envoy image
docker build -t envoy-wrapper:test -f Dockerfile .
kind load docker-image envoy-wrapper:test --name ai-gateway-lab
kubectl apply -f gatewayparams.yaml

# Test
kubectl port-forward -n kgateway-system svc/ai-gateway 8082:8080
```
In another terminal:
```bash
curl -v -X POST http://localhost:8082/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model":"gpt-4","messages":[{"role":"user","content":"Hello"}]}'
```
Expected output:
```http
< HTTP/1.1 200 OK
< content-type: application/json
< server: envoy
<
{
  "choices": [{
    "message": {
      "content": "This is a mock chat response from httpbun..."
    }
  }]
}
```
**Note**: The X-Custom-Transformed: true header is present in the code on GitHub. The Docker image in this lab was built before the header was added. To see the header in action, rebuild the image with the updated Rust module.

## Key Files

| File | Purpose |
|------|---------|
| `gateway.yaml` | Entry point |
| `httpbun-route.yaml` | URLRewrite filter |
| `gatewayparams.yaml` | Custom image deployment |
| `rust/src/http_simple_mutations.rs` | Custom header logic |

## Note on kgateway v2.3

This lab was built and tested with kgateway **v2.2.x**, which is stable and fully supported.

kgateway v2.3 introduces breaking changes to the Rust dynamic module system (the filter name changed from `http_simple_mutations` to `rustformation`).

- For detailed migration: [`adding-a-filter.md` guide](https://github.com/kgateway-dev/kgateway/blob/main/internal/envoy_modules/adding-a-filter.md)
- For the new filter skeleton: [`kgateway-example-filter`](https://github.com/kgateway-dev/kgateway/tree/main/internal/envoy_modules/filters/kgateway-example-filter)

## Resources

- [kgateway Documentation](https://kgateway.dev/docs/)
- [Gateway API](https://gateway-api.sigs.k8s.io/)
- [httpbun - Mock LLM](https://github.com/sharat87/httpbun)

## License

Apache 2.0
