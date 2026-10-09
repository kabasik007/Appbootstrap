---
applyTo: "**"
---
# Testing and regression instructions
Every behavior change should have executable acceptance tests or a documented manual verification method if automation is impractical. Test normal paths, failures, invalid input, race/cancellation cases, cleanup and compatibility as relevant. Run the exact available checks, state which checks could not run, and never write fabricated "all passed" results. No fake no-op tests to make CI green.

