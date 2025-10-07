### Technical Tasks for First Iteration: Pingora LB with Auth Middleware & Embedded Cache

This iteration focuses on a minimal viable implementation of the Pingora load balancer (LB) as the gateway, integrated with basic auth middleware (using auth-framework for JWT/session validation) and Moka for embedded in-memory caching (e.g., sessions/Tokens). Scope: Single-node PoC, HTTP proxying to a mock backend, no HA/HL scaling, PQC, or advanced features yet. Target: <100ms end-to-end latency for auth-protected requests; 1k RPS throughput.

#### Prerequisites (Setup Phase: 1-2 Days)
1. **Environment Setup**:
    - Install Rust 1.80+ and Cargo.
    - Create a new Cargo workspace: `cargo new pingora-auth-poc --bin`.
    - Add dependencies in `Cargo.toml`:
      ```
      [dependencies]
      pingora = "0.3"  # Latest stable for proxy
      pingora-proxy = "0.3"
      auth-framework = "0.4"  # For middleware
      moka = { version = "0.12", features = ["future"] }  # Embedded cache
      tokio = { version = "1", features = ["full"] }
      serde = { version = "1", features = ["derive"] }
      jsonwebtoken = "9"  # JWT utils
      tower = "0.4"  # For middleware chaining
      tracing = "0.1"  # Logging
      ```
    - Set up a mock backend (e.g., simple Axum server on localhost:3001 echoing requests).

2. **Project Structure**:
    - Organize: `src/main.rs` (entrypoint), `src/proxy.rs` (Pingora config), `src/auth_middleware.rs` (auth logic), `src/cache.rs` (Moka setup).

#### Implementation Phase (3-5 Days)
3. **Pingora LB Core Setup**:
    - In `src/proxy.rs`: Configure basic HTTP proxy with TLS termination (rustls for localhost self-signed cert).
        - Listen on 0.0.0.0:443; proxy all `/api/*` to mock backend.
        - Enable HTTP/2 and connection pooling.
        - Task: Write `ProxyHttp` service; test basic forwarding with `curl https://localhost/api/test`.

4. **Embed Moka Cache**:
    - In `src/cache.rs`: Create an Arc-shared `Cache<String, String>` with TTL=15min, max_capacity=10k entries.
        - Methods: `async get_session(key: &str) -> Option<String>`, `async insert_session(key: &str, value: String)`.
        - Encrypt values with a simple AES key (from env; placeholder for Vault).
    - Task: Integrate into main: Pass `Arc<Cache>` to proxy context; test insert/get with a simple Tokio task.

5. **Auth Middleware Implementation**:
    - In `src/auth_middleware.rs`: Build a Tower service for pre-proxy hook.
        - Extract `Authorization: Bearer <token>` from headers.
        - Validate JWT with auth-framework's `verify_token` (claims: sub, exp, iat); fallback to session lookup in Moka.
        - On valid: Proceed; invalid/missing: Return 401 with WWW-Authenticate header.
        - Cache validated sessions (key: `session:{user_id}`, value: serialized claims).
    - Task: Register as `add_pre_hook` in Pingora; test with a generated JWT (use `jsonwebtoken::encode` for PoC).

6. **Integration & Basic Flows**:
    - In `src/main.rs`: Wire components—init cache, start proxy with middleware, run Tokio runtime.
        - Add a simple `/auth/login` endpoint (mock: issue JWT on POST with dummy creds).
    - Task: End-to-end test: Login → Get token → Protected API call (cached hit/miss).

#### Testing & Validation Phase (1-2 Days)
7. **Unit/Integration Tests**:
    - Use `#[tokio::test]` for async: Mock requests, assert middleware rejects invalid tokens, cache TTL expiry.
    - Task: Cover 80% with `cargo test`; include tracing spans for debug.

8. **Performance & Security Smoke Tests**:
    - Load: Use `wrk` or `k6` for 1k RPS on protected endpoint; measure p99 <100ms.
    - Security: Test replay (blacklist in cache), poisoning (sanitize keys before insert).
    - Task: Run locally; log metrics (e.g., cache hit rate >70%).

#### Deliverables & Next Steps
- **Output**: Runnable binary (`cargo run`) serving protected proxy; basic README with setup/curls.
- **Risks/Mitigations**: Pingora config quirks—use examples from repo; auth edge cases—mock auth-framework calls.
- **Iteration 2 Teaser**: Add WASM frontend serving and Vault integration.

# TODO

1. Connect with secret manager
2. Rate limiter + cache
2. Circuit braker
3. CORS
4. Abstragate middleware: Auth<Session<Protocol>>, Session<Protocol>, Downstream / Upstream <Protocol>
5. Metric exporting