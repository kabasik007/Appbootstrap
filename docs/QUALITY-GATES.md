# Quality gates

## Template repository checks
Run from this repository root:

    python3 scripts/validate.py
    python3 -m unittest discover -s tests -v

The GitHub Actions workflow runs these checks on push and pull request. They validate **bootstrap files and generator behavior**, not an app's runtime, UI or performance.

## Generated applications
Replace these categories with executable commands for the chosen stack:
1. Format/style and static linting.
2. Type checking/compilation (where applicable).
3. Unit tests for business rules and boundary adapters.
4. Integration tests for storage, device/API boundaries and error handling.
5. Build/package/startup smoke tests on supported target OSes.
6. Security/dependency checks and secret scanning.
7. Performance/soak checks against documented budgets.
8. Accessibility and usability checks for products with a UI.

A generated project is **not release-ready** while mandatory categories lack implementation or evidence. Do not add dummy CI checks that always succeed.

## Definition of done
- Acceptance criteria met, including failures and cancellations.
- Tests added/updated and actually run; results reported.
- Build and packaging verified on supported platforms, or clearly flagged unverified.
- No undocumented breaking change, secret, uncontrolled retry/queue, or severe performance regression.
- Documentation and migration/release notes updated when relevant.

