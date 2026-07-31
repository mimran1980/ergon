# Consuming Decode Stages

Groups and var-data are consumed in schema order. `finish()` hands the next
named stage back to you:

```text
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:demo_car_decode_stages}}
```
*(This code comes from the `sbe-feature-tour` sample crate.)*
