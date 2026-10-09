# Web application profile

**Best for:** browser-facing sites, web apps, dashboards and SaaS.

## Decide
- SSR/SPA/static/hybrid architecture from UX, SEO, offline needs and deployment constraints.
- Client/server authority, state boundaries, caching, data consistency, API versioning and pagination.
- Candidate ecosystems: React, Vue, Svelte, Laravel, Django, Go and others — never mandatory by default.

## Design
- Keep business permissions enforced server-side, not only in UI.
- Use accessible semantic interactions, keyboard support and internationalization as required.
- Avoid blocking the main browser thread and unnecessary JavaScript/hydration.
- Measure Core Web Vitals and application-specific latency with representative connections/devices.
- Plan auth, rate limits, XSS/CSRF protection, observability, resilient APIs and schema migrations.

## Verify
E2E workflows, web accessibility, mobile browsers, failure/slow-network behavior, security boundaries, backup and rollback.

