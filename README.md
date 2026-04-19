# Chainless-LB

Chainless-LB is a high-performance, edge-compatible application delivery network and proxy framework built in Rust on top of Cloudflare's `pingora` ecosystem. Designed as an extensible SDK rather than a monolithic binary, it provides static dispatching, kernel-level networking control, and compile-time optimization. It's build as an abstraction layer over pingora to provide user-friendly pre-built API for common use cases of HTTP/S, UDP, WireGuard, etc. with JWT/OIDC authentication integrated with any OAuth2 provider. Aside of TLS/SSO pingora abstractions it provides unique CDN features like cross-zone routing and multi-region caching with authentication in federated or hybrid cloud environments. See comparison with other proxies in [Why-Pingora](./README.md#Why-Pingora).

## Core Use Cases

- Native and robust proxying for HTTP/1.0, HTTP/2.0, and HTTP/3.0 streams.
- **UDP WireGuard Proxying** for secure ingress gateway for virtualized environments, VPN infrastructure and cross-cloud or hybrid connectivity.
- **Cross-Zone Networks** for horizontally scalable routing and seamless geographic traffic distribution natively across fragmented or multi-cloud network topographies or CDNs.
- **Protocol Extensibility** beyond HTTP and UDP to support arbitrary specialized TCP/UDP protocols via the pure-Rust trait-based pipeline.

## Authentication & Identity

The proxy strictly enforces a **Stateless Authentication Model** designed to protect edge latencies:

- **Zero Database Lookups**. Auth flows utilize local JWT cryptographic payload verification instead of blocking on synchronous session database transactions.
- **OIDC Integration**. Natively supports OpenID Connect logic for distributed identity management.
- **Verification Caching**. Cryptographic validations are temporarily cached in-memory. If a parsed token is deemed legitimate, subsequent requests bypass duplicate asymmetric cryptographic math.
- **Extensible Providers**. Because authentication is managed via a static `AuthProvider` trait, extending the pipeline to hook into arbitrary custom identity providers (e.g., ORY architectures or custom internal PKI) is trivial.

## Architecture & Design Pillars

1. **High-Performance execution** Fully async, built on `tokio` and `pingora`, executing in latency categories often reserved for kernel subsystems and C++.
2. **Compile-Time Configuration Dispatch:** Middleware execution layers (rate limits, caching, circuit breakers) and auth validations are mapped into generic types. This eliminates runtime allocations associated with trait objects (like `Box<dyn Middleware>`), allowing the Rust compiler to aggressively inline and layout proxy execution at compile time.
3. **SDK Approach:** Chainless-LB is not a black-box daemon. It is provided as a workspace library (`chainless-lb-backend`), allowing internal teams to embed it directly within custom gateway applications (see `/examples/`).
4. **Edge Compatible:** Minimal compute footprints and low memory consumption make it ready to deploy natively at the edge, whether in massive bare-metal clusters or constrained edge-PoPs.
5. **In-Memory Caching:** Heavy reliance on generic concurrency caches (e.g., `moka`) drastically reduces synchronous round-trips to upstream persistence layers.
6. **DNS-Based Scalability:** Favors cloud-native standard horizontal scaling capabilities via DNS-based upstream discovery instead of stateful topology sharing.

## CDN & Edge Deployments

Chainless-LB is purposfully built as the foundational engine for **Content Delivery Networks (CDNs)** and isolated Edge-PoPs (Points of Presence) deployed across Anycast networks:

- **Zero-State Topologies**. By enforcing stateless JWT authentication and depending solely on DNS infrastructure for upstream tracking, individual proxy nodes carry zero global state. This fundamentally aligns with CDN nodes dropping and joining BGP routes globally.
- **Aggressive Edge Caching**. Designed around heavily concurrent LRU (`moka`) caching semantics, assets can be persisted locally within RAM per edge PoP with negligible eviction delays, guaranteeing high edge offload ratios.
- **Resource Constraints**. Compiled as a single static binary with statically dispatched middleware mappings, it demands exceptionally low hardware footprints—allowing it to run comfortably in constrained/embedded edge environments deep into global ISP networks.

## Experimental Features

- **PQC-TLS (Experimental)** Preemptively engineered for future-proof security environments, the proxy incorporates experimental support for Post-Quantum Cryptography (PQC) integration over TLS. It leverages native wrappers to test and adapt to quantum-hard encryption standards before they become mandatory compliance metrics.
- **eBPF Payload Enforcement** Capable of natively loading pure-Rust XDP eBPF programs via `aya` into kernel space, dropping malicious traffic before it ever hits the user-space proxy socket.

## Configuration & Source of Truth

To complement compile-time pipeline static-dispatch, all runtime state configuration (such as ports, buffering limits, routing topologies, and upstream IP resolution) uses cleanly structured **TOML specifications** acting as single sources of truth.

These configurations (like `config.prod.toml` mapping out kernel `tcp_recv_buf` optimizations) completely dictate proxy behavior upon execution, keeping the binary strictly isolated from mutating environment states. They are directly embedded during integration tests to guarantee identical behaviors in CI/CD matching production metrics.

---

### Workspace Operations

Chainless-LB leverages a decoupled Cargo workspace structure. For building and testing instructions across the various architectural deployments, refer to the [Examples Documentation](examples/README.md).

```bash
# Global Workspace Commands

# Compiles the core isolated backend and every standalone edge proxy example securely
cargo build-all

# Runs integration validations (including eBPF hooks and auth parsers) alongside unit tests
cargo test-all

# Dispatches the 'Goose' native engine to stress test your local deployment's HTTP/2 logic
cargo bench-all

# Drops into a Linux Network Namespace configuring virtual Ethernet pairs for eBPF sandbox validation
cargo sandbox
```

## Feature Matrix

| Category          | Feature                    | Status | Implementation Detail                                                       |
| :---------------- | :------------------------- | :----: | :-------------------------------------------------------------------------- |
| **Routing**       | HTTP 1/2 Proxying          |   ✅   | Full Pingora core integration with native connection pooling.               |
| **Routing**       | HTTP 3 Proxying            |   ⏳   | In progress                                                                 |
| **Routing**       | WireGuard UDP Proxying     |   ⏳   | In progress                                                                 |
| **Routing**       | Cross-Zone Anycast Support |   ❓   | DNS-based horizontal upstream discovery, zero global state (needs testing). |
| **Performance**   | Static Pipeline Dispatch   |   ✅   | `PipelineBuilder` eliminates runtime `Box<dyn>` overhead entirely.          |
| **Caching**       | In-Memory LRU Cache        |   ✅   | `moka` integration natively offloading upstream responses.                  |
| **Caching**       | External Distributed Cache |   ⏳   | `CacheTrait` adapters for Redis/Dragonfly mapped for future iterations.     |
| **SSO proxy**     | Stateless JWT Verification |   ✅   | Async cryptographic validation bypassing DB roundtrips.                     |
| **Policy proxy**  | Federation / RBAC / OIDC   |   ⏳   | Deep integrations with ORY Keto / Dex planned.                              |
| **Security eBPF** | eBPF XDP Payload Drops     |   🧪   | Network-level sandboxed package-dropping natively evaluated via `aya`.      |
| **Security PQC**  | PQC-TLS (Post-Quantum)     |   ⏳   | Foundational integration mapped, awaiting cryptographic normalization.      |
| **Security OPA**  | Open Policy Agent (OPA)    |   ⏳   | Policy-as-Code abstractions to enforce granular edge access scopes.         |
| **Observability** | OTLP Telemetry Export      |   ✅   | Embedded Prometheus metric streaming via gRPC / HTTP.                       |
| **Observability** | FinOps & Anomaly Detection |   ⏳   | Automated cost-routing / alerts planned for future expansions.              |

_(✅ = Ready natively inside Workspace / 🧪 = Experimental Phase / ⏳ = Roadmap)_

## Why Pingora?

Chainless-LB is built on **Pingora** natively in Rust (by [Cloudflare](https://github.com/cloudflare/pingora)) rather than relying on legacy daemons or raw network primitives. Here is how it compares to the broader ecosystem:

- **vs NGINX / HAProxy (C/C++):** NGINX is a standalone daemon historically burdened by memory-safety concerns and a difficult C-module plugin architecture (often relying on Lua scripting). Chainless-LB acts as a pure-Rust **embeddable SDK**. This allows teams to construct mathematically secure, memory-safe proxies dynamically, replacing slow interpreted scripts with zero-cost compiled abstractions. See Cloudflare migration [from NGINX to Pingora](https://blog.cloudflare.com/how-we-built-pingora-the-proxy-that-connects-cloudflare-to-the-internet/).
- **vs Hyper / Tower (Rust):** Hyper is a low-level HTTP state machine. However, building a proxy on Hyper requires extra effort to re-implement L7 primitives (upstream connection pooling, active TCP health-checking, circuit breaking) entirely from scratch. Pingora provides these battle-tested proxy lifecycle objects out-of-the-box.
- **vs Sōzu (Rust):** Sōzu is an impressive reverse proxy capable of hot-reloading server configurations, but it is fundamentally structured as a monolithic daemon. Chainless-LB favors Pingora's API-first SDK approach, allowing embedding parsing logic directly into internal custom gateways.
- **vs Linkerd2 Proxy (Rust):** Linkerd is a Rust proxy optimized natively for Kubernetes microservices, but its proxy architecture is inextricably coupled to its own service mesh control plane. Chainless-LB operates completely independently, making it ideal for isolated bare-metal ISP boxes, VPN nodes, and Anycast-routed CDNs.
