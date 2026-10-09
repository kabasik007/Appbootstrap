---
applyTo: "**"
---
# Security instructions
Follow `docs/SECURITY.md` and the project threat model. Treat files, network responses, IPC, plugins and user input as untrusted. Validate at boundaries, use least privilege, use parameterized queries, protect secrets, scope logs and avoid unsafe shell execution. Review dependency licensing and update chain of trust. Never commit tokens, keys, account data or private credentials.

