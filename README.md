# kgateway AI Gateway Lab

[![License](https://img.shields.io/badge/License-Apache%202.0-green.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.88+-orange?logo=rust)](https://www.rust-lang.org/)
[![kgateway](https://img.shields.io/badge/kgateway-2.2.x-purple)](https://kgateway.dev/)

Extend kgateway with a custom Rust module that adds `X-Custom-Transformed: true` header to every response.

## Project structure
```text
kgateway-ai-gateway-lab/
├── gateway.yaml
├── httpbun.yaml
├── httpbun-backend.yaml
├── httpbun-route.yaml
├── trafficpolicy.yaml              # Transformation headers
├── gatewayparams.yaml              # Custom image deployment
├── internal/
│   └── envoyinit/
│       └── rustformations/
│           ├── Cargo.toml
│           └── src/
│               ├── lib.rs
│               └── http_simple_mutations.rs   # Your Rust module
├── rust/                           # (Legacy - can be removed)
│   ├── rustformations/
│   └── transformations/
└── envoy.yaml
```

## Quick Start (v2.3

```bash
# Create cluster
kind create cluster --name ai-gateway-lab

# Install kgateway
kubectl apply -f https://github.com/kubernetes-sigs/gateway-api/releases/download/v1.4.0/standard-install.yaml

helm upgrade -i kgateway-crds oci://cr.kgateway.dev/kgateway-dev/charts/kgateway-crds --version v2.3.0-rc.1 --namespace kgateway-system --create-namespace

helm upgrade -i kgateway oci://cr.kgateway.dev/kgateway-dev/charts/kgateway --version v2.3.0-rc.1 --namespace kgateway-system

# Deploy mock LLM and Gateway resources
kubectl apply -f httpbun.yaml
kubectl apply -f httpbun-backend.yaml
kubectl apply -f gateway.yaml
kubectl apply -f httpbun-route.yaml

# Deploy custom Envoy image
kind load docker-image ghcr.io/kgateway-dev/envoy-wrapper:v1.0.1-dev --name ai-gateway-lab
kubectl apply -f gatewayparams.yaml

# Apply TrafficPolicy for custom headers
kubectl apply -f - <<EOF
apiVersion: gateway.kgateway.dev/v1alpha1
kind: TrafficPolicy
metadata:
  name: my-smart-header-filter
  namespace: kgateway-system
spec:
  targetRefs:
    - group: gateway.networking.k8s.io
      kind: HTTPRoute
      name: httpbun-route
  transformation:
    response:
      set:
        - name: X-Custom-Transformed
          value: "true"
        - name: X-Smart-Header
          value: "method=%REQ(:METHOD)%&path=%REQ(:PATH)%"
EOF

# Test
kubectl port-forward -n kgateway-system svc/ai-gateway 8082:8080
```
In another terminal:
```bash
curl.exe -v -X POST http://localhost:8082/v1/chat/completions -H "Content-Type: application/json" -d '{\"model\":\"gpt-4\",\"messages\":[{\"role\":\"user\",\"content\":\"Hello\"}]}'
```
Expected output:
```http
< HTTP/1.1 200 OK
< content-type: application/json
< x-custom-transformed: true
< x-smart-header: method=POST&path=/v1/chat/completions
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
## Key Files

| File | Purpose |
|------|---------|
| `gateway.yaml` | Entry point with kgateway GatewayClass |
| `httpbun-route.yaml` | HTTPRoute with URLRewrite and ExtensionRef |
| `trafficpolicy.yaml` | Transformation headers (X-Custom-Transformed, X-Smart-Header) |
| `gatewayparams.yaml` | Custom Envoy image deployment |
| `internal/envoyinit/rustformations/` | Custom Rust module source |


## Note on kgateway v2.3

This lab uses **kgateway v2.3.0-rc.1**.

kgateway's built-in Rust module is called `rustformation` (replacing the old `http_simple_mutations` from v2.2).

This lab does **not** use the built-in module. Instead, it demonstrates a **custom** Rust filter named `my-smart-header` — a user-defined extension. The filter is invoked via a `TrafficPolicy` with the `transformation` field, showing how you can add your own custom logic beyond what kgateway provides out of the box.

## Resources

- [kgateway Documentation](https://kgateway.dev/docs/)
- [Gateway API](https://gateway-api.sigs.k8s.io/)
- [httpbun - Mock LLM](https://github.com/sharat87/httpbun)

## License

Apache 2.0
