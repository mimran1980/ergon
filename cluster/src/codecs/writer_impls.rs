//! `Writer` impls that sbe-tool 1.39.0's Rust target omits.
//!
//! sbe-tool emits `pub trait Writer<'a> { fn get_buf_mut(&mut self) -> &mut
//! WriteBuf<'a>; }` and `pub struct WriteBuf<'a>` for every schema, but never
//! generates `impl Writer for WriteBuf`. The generated encoders call
//! `get_buf_mut`, so without these impls they fail to compile.
//!
//! Defining the impls here — instead of hand-editing each generated `mod.rs` —
//! keeps the generated files pure sbe-tool output, so `just
//! generate-cluster-codecs` reproduces them exactly (no drift from hand-edits).

use super::{cluster_codecs, cluster_codecs_mark, rfq_codecs};

impl<'a> cluster_codecs::Writer<'a> for cluster_codecs::WriteBuf<'a> {
    #[inline]
    fn get_buf_mut(&mut self) -> &mut cluster_codecs::WriteBuf<'a> {
        self
    }
}

impl<'a> cluster_codecs_mark::Writer<'a> for cluster_codecs_mark::WriteBuf<'a> {
    #[inline]
    fn get_buf_mut(&mut self) -> &mut cluster_codecs_mark::WriteBuf<'a> {
        self
    }
}

impl<'a> rfq_codecs::Writer<'a> for rfq_codecs::WriteBuf<'a> {
    #[inline]
    fn get_buf_mut(&mut self) -> &mut rfq_codecs::WriteBuf<'a> {
        self
    }
}
