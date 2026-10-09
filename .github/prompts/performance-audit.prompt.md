---
description: Measure responsiveness, memory, startup and bottlenecks before optimizing
---
# Performance audit

Follow `docs/PERFORMANCE.md`. Scope: `\${input:area:Which workload or screen to profile}`

- Record hardware, OS, build type, dataset, workload and tools.
- Establish baseline metrics and a repeatable test; clearly mark unavailable measurements.
- Trace hot paths and identify blocking UI calls, unbounded work/queues, memory growth and excessive I/O.
- Rank measured bottlenecks by user impact and implementation risk.
- Make one limited optimization per hypothesis, remeasure and compare regressions.
- Report real p50/p95/p99 or relevant time/memory data. Do not assert "faster" without evidence.

