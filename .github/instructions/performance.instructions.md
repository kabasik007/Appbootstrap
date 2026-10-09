---
applyTo: "**"
---
# Performance instructions
Follow `docs/PERFORMANCE.md` and project `docs/PERFORMANCE_BUDGET.md`. Define workload/hardware, baseline before tuning, and measure p95 latency, start time, memory and appropriate throughput. No blocking I/O or heavy compute on UI/event loops. No blocking operations, unpredictable allocation or unbounded logging in hard real-time callbacks. Bound queues/caches/retries; report measurements and methodology, not adjectives.

