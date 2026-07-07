# Justfile consolidation

## Status: DONE

## What
Two just files existed simultaneously: `.justfile` (1.3KB, complete) and
`justfile` (733B, partial). `just` v1.55+ errors when both files exist:
"multiple candidate justfiles found".

## Fix
Deleted `justfile`, kept `.justfile` (the more complete version with `set
shell`, `default` recipe, `ci-status`, `disable-bounds-checks` in `test`,
and `cargo-udeps` in `deps`).

## Verification
```sh
just --list  # shows all recipes
```
