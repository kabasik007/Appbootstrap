# Desktop WebView profile

**Best for:** desktop experiences that benefit from web UI skills and reusable UI components.

## Candidate stacks
Tauri, Electron and other supported WebView shells are examples. Choose based on footprint, required native APIs, support policy and actual benchmarks, not brand preference.

## Design and security
- Use a **typed and allowlisted** UI ↔ native command boundary.
- Treat web content, external URLs, deep links, plugins and clipboard as untrusted.
- Avoid exposing unrestricted shell/filesystem access to the renderer.
- Profile time to usable UI, WebView memory, bundle weight, IPC latency and idle CPU.
- Offload heavy I/O and computation from the renderer/main event loops.
- Plan auto-updates, signatures, permissions and multiple OS packaging explicitly.

## Release checks
CSP, devtools exposure, navigation handling, IPC authorization, offline handling, memory soak, screen scaling, app startup and update rollback.

