# Trust Boundary

| API | When |
|-----|------|
| `try_from` / `try_wrap_and_apply_header` / `try_*` | **Untrusted** buffers (network, file, other process) |
| `wrap` / trusted companions | Buffer already validated (or built by you this turn) |
| `verify` | Walk the full dynamic tail before trusting accessors |
