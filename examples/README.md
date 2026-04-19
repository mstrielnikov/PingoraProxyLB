# Chainless-LB Examples

This directory contains self-contained, standalone demonstrations of the High-Performance Computing proxy built on top of the generic `chainless-lb-backend`.

Each example validates a different architectural layer (Authentication, Kernel eBPF, Telemetry, Cryptography, etc.) and exposes a fully decoupled `Cargo.toml`. Since they are all workspace members, you can execute them directly via our global aliases.

---

### 1. Full Server Base Demo (`full_server`)

Demonstrates the fundamental proxy lifecycle. Mocks a generic TCP backend connection without imposing exotic middleware restrictions. Includes local integration tests for basic HTTP passthrough.

- **Build**: `cargo build -p example-full-server`
- **Run**: `cargo run -p example-full-server`
- **Test**: `cargo test -p example-full-server` (executes `/tests/integration.rs`)

### 2. High-Performance Static Dispatch (`static_dispatch`)

Instead of dynamically passing middleware instances (`Arc<dyn ErasedPipeline>`) at runtime, this shows how to structurally type-lock things like `RateLimit` or `NoOpAuth` at compile-time. This entirely eliminates heap allocation overhead per request, making it the highest throughput design for latency-critical HPC environments.

- **Build**: `cargo build -p example-static-dispatch`
- **Run**: `cargo run -p example-static-dispatch`

### 3. JWT Proxy Authentication (`jwt_auth`)

A security implementation natively offloading authentication parsing from upstream systems. Leverages asynchronous `reqwest` checks and local RS256 `jsonwebtoken` claims parsing natively within Pingora.

- **Build**: `cargo build -p example-jwt-auth`
- **Run**: `cargo run -p example-jwt-auth`

### 4. Post-Quantum Cryptography TLS (`tls_pqc`)

Demonstrates future-proof connection handling by utilizing `rustls` with quantum-hard algorithms over OpenSSL wrappers. It dynamically mounts `config.toml` defined synthetic certificates.

- **Build**: `cargo build -p example-tls-pqc`
- **Run**: `cargo run -p example-tls-pqc`

### 5. OpenTelemetry & Metrics Export (`metrics_export`)

Shows the proxy emitting deep analytical traces and histogram metrics in OpenTelemetry format (`OTLP` payload). Automatically routes the telemetry payload to standard collectors out-of-band so it never degrades live web traffic.

- **Build**: `cargo build -p example-metrics-export`
- **Run**: `cargo run -p example-metrics-export`

### 6. gRPC Payload Forwarding (`grpc_forward`)

An edge proxy capability demonstrating raw bidirectional HTTP/2 `gRPC` streams. Demonstrates parsing Protobuf encoded routing without breaking the payload stream.

- **Build**: `cargo build -p example-grpc-forward`
- **Run**: `cargo run -p example-grpc-forward`

### 7. RPS Throughput Benchmark (`rps_benchmark`)

A sophisticated built-in attack environment. Leverages [Goose](https://github.com/tag1consulting/goose) natively inside Rust to attack the locally spawned load balancer with immense traffic configurations.

- **Build**: `cargo build -p example-rps-benchmark --release`
- **Run via Cargo Alias**: `cargo bench-all -u 100 -r 10 -t 5s`

---

### Global Workspace Commands

To test and validate the integrity of every scenario above, run the native Cargo aliases deployed at the root directory:

```bash
cargo build-all   # Compiles the whole suite
cargo test-all    # Triggers all internal /tests integrations + unit modules
cargo bench-all   # Synthesizes Goose Load
cargo sandbox     # Wraps your current process in Linux namespaces for safe eBPF testing
```
