# Security baseline

Security controls are proportionate to the project's trust boundaries and deployment model. Before implementation, fill `docs/THREAT_MODEL.md`.

## Mandatory practices
- Validate and normalize untrusted input at every entry boundary.
- Use least privilege for filesystem, camera, microphone, network, database and cloud credentials.
- Keep passwords, API keys, tokens and signing credentials out of repos, prompt contexts, logs and build artifacts.
- Prefer safe APIs, parameterized queries and well-maintained dependencies.
- Document how sensitive data is stored, encrypted where appropriate, retained, backed up and deleted.
- Define update authenticity and release-signing processes for distributed executables.
- Prevent injection in shell execution, HTML rendering, database access and file paths.
- Record critical security decisions and dependency licenses; review transitive packages.
- Use structured error handling: useful diagnostics for developers, no secrets or internal traces leaked to end users.

## Desktop and offline apps
Consider local permissions, auto-update chain of trust, file-type parsing, malicious plugins, credential storage, IPC commands and untrusted media/documents.

## Web, mobile and backend
Consider authentication, session lifecycle, authorization per action, CSRF/XSS, rate limiting, quotas, transport security and access audit, proportional to risk.

## Never assume
A pure local app is not automatically secure; a cloud app is not automatically insecure. Model threats and test relevant attack surfaces. Escalate issues involving sensitive data or destructive operations.

