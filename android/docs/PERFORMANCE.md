# Android performance contract (budgets must be measured)

This starter intentionally has **no invented benchmark results**.

Measure on a specified baseline Android phone and build type:
- cold / warm start; time to interactive,
- slow/frozen frames and jank (Perfetto / Macrobenchmark),
- RAM / allocations / GC under normal and long-running sessions,
- wakeups, CPU, battery and network usage,
- offline/low-memory rotation/recreation,
- large lists or media decoding workloads as applicable.

General engineering rules:
- Keep I/O off the main thread; use structured coroutines and cancellation.
- Avoid unbounded lists, retries, workers, events, caches and threads.
- Use stable keys in Compose Lazy lists. Don't allocate expensive objects in hot recomposition.
- Isolate background audio services and player lifecycle from UI; never block audio callbacks.
- Consider Baseline Profiles and ProfileInstaller after real app startup paths exist.
- Treat emulator timings as diagnostics, not proof of device performance.
- Record before/after test devices, runs, build hashes and uncertainty in PRs.
