//! er͏go-clickhouse-persist-derive — todo 04.
//!
//! Proc-macro for `#[derive(Persist)]`.
//! Stub: no-op until the subagent fills it in.

use proc_macro::TokenStream;

/// Derive the [`Persist`] trait for a struct.
///
/// Stub — returns an empty impl that panics at runtime.
#[proc_macro_derive(Persist, attributes(persist))]
pub fn derive_persist(_input: TokenStream) -> TokenStream {
    // Stub — todo 04 replaces this
    TokenStream::new()
}
