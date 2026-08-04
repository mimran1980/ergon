# Multi-Template Dispatch

`AnyMessage` reads the generated header layout and dispatches on the template
ID. Prefer **`AnyMessage::try_decode`** at untrusted boundaries (returns
`Result`). Bare `AnyMessage::decode` is the same implementation today; keep
using `try_*` naming for consistency with the three-tier trust boundary.

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:demo_any_message}}
```
*(This code comes from the `sbe-feature-tour` sample crate.)*
