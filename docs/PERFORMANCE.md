# Performance and responsiveness playbook

## Performance is a contract, not a feeling
Choose **target hardware, OS version, dataset size, power mode, network condition, build type, warm/cold state and measurement methodology** before stating budgets. Example numbers in other products are not guarantees for this project.

Recommended metrics (choose relevant ones):
- cold and warm start: time to usable interaction
- UI interaction latency: p50/p95/p99, worst-case stalls, dropped frames
- steady and peak memory, memory growth during stress/soak
- CPU in idle and active workloads; battery/energy if mobile
- I/O volume, database query timings, cache hit ratios
- throughput, queue depth, cancel/shutdown latency
- error rate, crash recovery, corruption tolerance

Write target and evidence in `docs/PERFORMANCE_BUDGET.md`. Use **TBD** for unknown targets.

## Design for responsiveness
- Never block UI/render threads on disk, network or CPU-heavy operations.
- Keep expensive work off real-time callbacks. Preallocate buffers where required; send data through bounded queues.
- Separate request/response, worker and rendering lifecycles. Propagate cancellation.
- Prefer pagination, streaming and incremental indexing to loading everything at once.
- Cache deliberately with explicit limits, eviction, stale-data semantics and invalidation.
- Bound retries, concurrency, logs and memory; handle backpressure.
- Avoid unnecessary framework/runtime startup work and reflection-heavy paths until measured.

## Optimize scientifically
1. Reproduce with fixed inputs and comparable builds.
2. Capture a baseline using profiling/tracing tools for the target stack.
3. State a hypothesis and change one bottleneck at a time.
4. Rerun and compare distributions; check correctness, memory and CPU regressions.
5. Keep benchmark scripts, hardware notes and representative fixtures with the app.

## Media-specific addition (optional)
For players, workstations and live media apps: test buffer underruns, device changes, large playlists, seek/cancel races, corrupted files, audio callback timings and long sessions. Native audio threads are usually stricter than UI event loops; never treat them as ordinary worker threads.

## Claims
Do not write "instant", "zero lag", "low memory" or a numerical guarantee without a measurement and test environment. Label hypothetical numbers as illustrative, not verified.

