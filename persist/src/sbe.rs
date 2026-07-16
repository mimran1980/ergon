//! Generated SBE types for dynamic schema/row messages (V1 + V2).
//!
//! V1 (`DynamicSchema`/`DynamicRow`, template IDs 1/2) is included at the
//! module root, preserving the flat import paths existing code expects.
//! V2 (`DynamicSchemaV2`/`DynamicRowV2`, template IDs 3/4, with Decimal
//! array support) lives in the [`v2`] submodule. Both are generated at
//! build time into `OUT_DIR` by `build.rs`; nothing is checked in.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(clippy::duplicated_attributes)]
// ponytail: generated SBE code triggers many clippy lints; suppress at the
// module boundary rather than fighting per-line in generated output.
#![allow(
    clippy::identity_op,
    clippy::eq_op,
    clippy::needless_borrow,
    clippy::manual_range_contains,
    clippy::missing_safety_doc,
    clippy::unnecessary_cast,
    clippy::redundant_closure,
    clippy::double_must_use,
    clippy::items_after_statements,
    clippy::struct_excessive_bools,
    clippy::only_used_in_recursion,
    clippy::too_many_arguments,
    clippy::type_complexity,
    clippy::comparison_to_empty,
    clippy::unnecessary_literal_unwrap,
    unexpected_cfgs,
    clippy::approx_constant,
    clippy::let_unit_value,
    clippy::redundant_clone,
    clippy::useless_vec,
    clippy::absurd_extreme_comparisons,
    clippy::question_mark,
    unused_braces,
    unused_assignments,
    unused_comparisons
)]

// V1: DynamicSchema (template 1) + DynamicRow (template 2) — flat imports.
include!(concat!(env!("OUT_DIR"), "/persist_sbe.rs"));

/// V2 codecs: `DynamicSchemaV2` (template 3) and `DynamicRowV2` (template 4)
/// with Decimal array support.
pub mod v2 {
    include!(concat!(env!("OUT_DIR"), "/persist_sbe_v2.rs"));
}
