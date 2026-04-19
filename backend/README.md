# Chainless-LB Backend (Library SDK)

This directory houses `chainless-lb-backend`, the core library crate of the Chainless-LB workspace.

Unlike traditional monolithic proxies, the backend is purposefully developed as an **embeddable edge SDK**. It abstracts away the heavy lifting of `pingora`, OpenTelemetry, circuit breaking, and protocol decoding into highly modular Rust traits. It enables you to consume this library to build your own custom edge topology or gateway endpoints (refer to the `/examples/` directory for robust implementations).

## Core SDK Features

1. **Static Pipeline Dispatching:**
   - Uses zero-cost abstractions by type-locking execution pipelines at compile time via the `PipelineBuilder` and `ErasedPipeline` mapping.
   - Middlewares like Rate Limiters (`rate_limit.rs`) and Circuit Breakers (`circuit_breaker.rs`) are executed rapidly without runtime `Box<dyn ...>` allocation overhead.

2. **Kernel-Space Hooks (eBPF):**
   - The `/ebpf/` module relies on the `aya` framework to natively inject safe, Rust-compiled XDP filters directly into the Linux kernel stack.
   - Provides mechanisms like `block_ip()` and `allow_ip()` outside of user-space traffic limits.

3. **Pluggable Architecture:**
   - **Authentication:** Controlled by the `AuthProvider` trait. The `NoOpAuth` provider enables raw passthrough, while custom handlers can seamlessly verify JWTs, delegate to ORY networks, or execute local PKI checks without altering the core LB runtime.
   - **Caching:** Governed by the `CacheTrait`. Natively integrates `moka` (a heavily concurrent LRU cache paradigm) but is designed to allow custom SSD / persistence traits mapped directly into the internal Pingora router.

4. **Deep Observability:**
   - The `observability/` engine dynamically routes traffic into `/var/log` targets (using `tracing-appender` for non-blocking production output) or local streams based on TOML parameters.
   - Natively connects `prometheus` histogram payloads back out as seamless OTLP formats (gRPC or HTTP).

## Integration Guide

To consume the `chainless-lb-backend` inside a custom Rust application, simply add it to your `Cargo.toml`:

```toml
[dependencies]
chainless-lb-backend = { path = "../backend" }
```

You can then natively initialize the static topology using `config::load_config()` to acquire your TOML payload, and securely dispatch Pingora structures over traits logic:

```rust
use chainless_lb_backend::proxy::LB;
use chainless_lb_backend::middleware::PipelineBuilder;

// Assemble zero-cost structures statically matched with your TOML
let pipeline = PipelineBuilder::new()
    .with_rate_limit(10_000)
    .build();

// ... Insert into Pingora execution context natively
```

## Internal Dependencies Highlights

- **pingora (0.8.0):** The native `C++` NGINX replacement built by Cloudflare strictly handling multi-threaded event looping and downstream sockets.
- **moka (0.12):** Used for advanced cache abstractions avoiding heavy lock contentions in `CacheTrait`.
- **aya:** Replaces raw C bindings to deploy BPF routing configurations into the OS safely.
- **opentelemetry / tracing:** Handles edge-level non-blocking analytics reporting to minimize request latency hits.
