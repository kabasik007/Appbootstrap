# Backend and CLI profile

**Best for:** services, APIs, background workers, job processors, command-line tools and automation.

## Decide
- Single binary vs managed runtime, deployment, OS, scalability, fault tolerance and operational expertise.
- Candidate stacks: Rust, Go, Python, Node.js, PHP, Java, etc.
- Operational limits: RPS, queue depth, input size, memory, startup and shutdown requirements.

## Design
- Explicit configuration, typed input/output and nonzero exit statuses where appropriate.
- Timeouts, retries with budgets/jitter, idempotency, rate limiting and graceful shutdown.
- Structured logs with redaction, metrics and health probes where relevant.
- Keep domain logic independent of transport, storage and external APIs.
- Use transactional boundaries and migrations for durable state.

## Verify
Bad flags/input, missing config, failure injection, SIGTERM/restart, slow dependencies, concurrency and soak, permission/secret handling, reproducible packaging.

