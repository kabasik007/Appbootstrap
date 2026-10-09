# Security, legality and distribution checkpoints

This is a software engineering plan, not a legal opinion. Before distributing binaries, perform a proper license and jurisdiction/terms review.

## Media downloads and rights
- Downloads must be limited to media users own or are authorized to save, and actions consistent with applicable law and service terms.
- Technical extractor support does **not** equal permission to download.
- No DRM/paywall bypass, credential harvesting, forced geo-restriction bypass or unauthorized account access.
- Platform-specific policy may restrict downloading or extracting audio; e.g. YouTube's public developer policies prohibit API clients providing downloads/audio extraction outside approved offerings. Official YouTube help distinguishes downloading a user's own uploads from Premium offline viewing.
- UI should explain that source restrictions and permissions are the user's responsibility, with clear warnings instead of claims of guaranteed support.
- Do not promote the app as guaranteed to download "all YouTube videos" or every source.

## Process execution
- yt-dlp/ffmpeg/JS runtime only from approved explicit executable paths, never through a shell command string.
- Validate schemes (http/https), output folders, file paths, disk space, length caps, filename templates and tool versions.
- Do not forward unsanitized URL parameter values to logs, process arguments other than intentional URL, or telemetry.
- Run untrusted fetchers in least-privilege process context; restrict child resources where feasible.
- Downloaded files are untrusted. Use bounded parsers and a test suite of corrupted media fixtures.

## Optional authentication
Do not silently read browser cookies. Authenticated-source support, if ever added, requires explicit consent, a documented retention model, secure secrets handling, and a separate review.

## Dependency licensing
- **Slint**: GPLv3, royalty-free-with-attribution, or commercial licenses with different obligations. Choose prior to shipping.
- **Symphonia**: MPL-2.0; comply with license and changes obligations.
- **yt-dlp**: upstream source is Unlicense, but Windows packaged executable may include GPLv3+ components. Verify the exact **artifact** being shipped, not just repo headline license.
- **FFmpeg**: build can be LGPL/GPL depending configuration, static/dynamic linking and enabled encoders. Distribution must carry corresponding notices and fulfill license duties.
- **JavaScript runtime/EJS**: add third-party notices and verify bundled license versions.
- Plug-in ecosystem introduces separate copyright/trademark/security issues; no plugin store in initial release.
- Product branding, icons, and skins must be original or correctly licensed; no AIMP/Winamp proprietary graphics without permission.

## Updates and integrity
- No silent arbitrary binary downloading from unknown sites. Only trusted update source, checksums/signatures, explicit consent/rollback.
- Windows installer signing plan and reputation/smart-screen expectations before public beta.
- Record reproducible versions, lockfiles, SBOM/licensing notices and automated dependency scanning once actual builds exist.

## Privacy
Offline-first by default, no account required. Avoid retaining whole private URLs, cookies, downloads history details in logs. Provide clear history deletion controls. Crash reports opt-in.

