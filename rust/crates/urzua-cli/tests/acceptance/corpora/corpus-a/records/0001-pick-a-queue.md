# 0001 — Pick a queue

> **Decision (Y-statement):** In the context of job fan-out,
> facing bursty producers that outrun consumers,
> we decided for a single durable queue
> and neglected per-tenant queues,
> to achieve smooth backpressure,
> accepting that one noisy tenant can delay the rest.

- **Status:** Accepted
- **Date:** 2026-01-05
- **Author:** pat

## Context

The body continues here and mentions a **Status:** word in prose.
