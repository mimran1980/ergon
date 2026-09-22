//! ergo-clickhouse-persist — prepared recording, dictionaries, ClickHouse ingest.
//!
//! A private, HFT-oriented persistence library: control-plane registration
//! once, then small bounded recording calls that borrow their inputs. A
//! recorder registers its tables and schema with the Archive-backed
//! catalog exactly once per session; every subsequent `persist_event!` call
//! is a bounded, allocation-free write keyed by that registration. The
//! `ingest` module (feature `ingest`/`archive`) replays those recordings
//! into ClickHouse on the consumer side.

#[allow(warnings)]
#[rustfmt::skip]
#[path = "generated/recording.rs"]
pub mod recording;

#[cfg(feature = "codegen")]
pub mod codegen;
#[cfg(feature = "config")]
pub mod config;
#[cfg(feature = "ingest")]
pub mod ingest;
pub mod persist;
pub mod protocol;
pub mod recorder;
pub mod registration;
pub mod schema;
