# Cluster Client

`ergo-aeron-cluster` is an experimental Rust client for Aeron Cluster. It uses
rusteron for transport and ergo-sbe-generated codecs for Aeron's Cluster
protocol.

> Hobby project. Do not use it as a production substitute for official Aeron
> Cluster client support.

## Scope

{{#include ../../../cluster/README.md:scope}}

## Features and harness

{{#include ../../../cluster/README.md:harness}}

## Limitations

Limitations, Aeron and rusteron versions, schema identities, failure modes and
what CI runs against a live cluster: [Compatibility](./compatibility.md).

Apache-2.0.
