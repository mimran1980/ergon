//! Inline sbe_rt emission, name helpers, composite/enum/set generation,
//! preamble, AnyMessage, FrameCursor, schema hash, and field metadata.

use crate::ir::{ByteOrder, Ir, Presence, PrimitiveType, Signal, Token};
use crate::structured_ir::*;
use proc_macro2::TokenStream;
use quote::format_ident;
use sha2::{Digest, Sha256};
use std::fmt::Write;

pub(crate) fn generate_sbe_rt_src() -> String {
    let module = quote::quote! {
        pub mod sbe_rt {
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub enum DecodeError {
                /// Buffer shorter than needed for `field` (`needed` vs `available` bytes).
                BufferTooShort { field: &'static str, needed: usize, available: usize },
                /// Wire `schemaId` does not match this codec (`expected` name/id vs `actual`).
                WrongSchema { expected: u16, actual: u16, expected_name: &'static str },
                /// Wire `templateId` does not match this message (`expected` name/id vs `actual`).
                WrongTemplate { expected: u16, actual: u16, expected_name: &'static str },
                /// Multi-template stream saw an id with no registered length/decoder.
                UnknownTemplateLength { template_id: u16 },
                /// Header field value exceeds the supported maximum for this platform.
                InvalidHeaderValue { field: &'static str, value: u64, maximum: u64 },
                /// Length-prefix for var-data exceeds schema max or platform size.
                InvalidVarDataLength { field: &'static str, length: u64, max_length: u64 },
                /// Field/group/data was added in a schema version later than the wire message.
                FieldNotInVersion { field: &'static str, wire_version: u16, since_version: u16 },
                /// Text var-data is not valid UTF-8.
                InvalidUtf8 { field: &'static str, error: core::str::Utf8Error },
                /// Text var-data is not valid ASCII.
                InvalidAscii { field: &'static str },
                /// Boolean wire enum was `NullVal` or an unknown discriminant.
                InvalidBoolean { field: &'static str, discriminant: u64 },
                /// Domain `try_*` conversion failed.
                DomainConversionFailed { field: &'static str, reason: &'static str },
            }

            impl core::fmt::Display for DecodeError {
                #[cold]
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    match self {
                        Self::BufferTooShort { field, needed, available } => write!(f, "field '{}': needed {} bytes, {} available", field, needed, available),
                        Self::WrongSchema { expected, actual, expected_name } => write!(f, "wrong schema: expected id {} ({}), got id {}", expected, expected_name, actual),
                        Self::WrongTemplate { expected, actual, expected_name } => write!(f, "wrong template: expected id {} ({}), got id {}", expected, expected_name, actual),
                        Self::UnknownTemplateLength { template_id } => write!(f, "unknown template id {}: SBE messages do not carry length. Use decode_frame() with an external frame length.", template_id),
                        Self::InvalidHeaderValue { field, value, maximum } => write!(f, "message header field '{}': value {} exceeds supported maximum {}", field, value, maximum),
                        Self::InvalidVarDataLength { field, length, max_length } => write!(f, "var data field '{}': length {} exceeds max {}", field, length, max_length),
                        Self::FieldNotInVersion { field, wire_version, since_version } => write!(f, "field '{}' not in wire version {} (added in version {})", field, wire_version, since_version),
                        Self::InvalidUtf8 { field, error } => write!(f, "field '{}': invalid UTF-8: {}", field, error),
                        Self::InvalidAscii { field } => write!(f, "field '{}': invalid ASCII", field),
                        Self::InvalidBoolean { field, discriminant } => write!(f, "field '{}': invalid boolean (discriminant {discriminant:#x})", field),
                        Self::DomainConversionFailed { field, reason } => write!(f, "field '{}': domain conversion failed: {}", field, reason),
                    }
                }
            }

            impl core::error::Error for DecodeError {
                fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
                    match self {
                        Self::InvalidUtf8 { error, .. } => Some(error),
                        _ => None,
                    }
                }
            }

            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub enum EncodeError {
                /// Encode buffer shorter than needed for `field` (`needed` vs `available`).
                BufferTooShort { field: &'static str, needed: usize, available: usize },
                /// Claim buffer length does not match ENCODED_LENGTH.
                ClaimLengthMismatch { expected: usize, actual: usize },
                /// Var-data payload longer than the schema max for `field`.
                VarDataTooLong { field: &'static str, max_length: usize, actual: usize },
                /// Fixed char/byte array source longer than the schema length.
                FixedArrayTooLong { field: &'static str, max_length: usize, actual: usize },
                /// ASCII fixed-array `*_str` received a non-ASCII `&str`.
                InvalidAscii { field: &'static str },
                /// Domain/DTO value outside the schema min/max range.
                ValueOutOfRange { field: &'static str, min: i128, max: i128, actual: i128 },
                /// Tried to write more group entries than the declared count.
                GroupFull { declared: u32, attempted: u32 },
                /// Known-size group closure returned without adding enough entries.
                GroupCountMismatch { declared: u32, actual: u32 },
                /// Unknown-size group entry count does not fit in `numInGroup`.
                GroupCountOverflow { maximum: u32, actual: u32 },
                /// Checked arithmetic overflow in encoded length computation.
                EncodedLengthOverflow,
                /// Domain `try_*` conversion failed.
                DomainConversionFailed { field: &'static str, reason: &'static str },
                /// Nested decode failure during encode/verify paths.
                Decode(DecodeError),
            }

            impl core::fmt::Display for EncodeError {
                #[cold]
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    match self {
                        Self::BufferTooShort { field, needed, available } => write!(f, "buffer too short for {field}: needed {needed}, available {available}"),
                        Self::ClaimLengthMismatch { expected, actual } => write!(f, "claim buffer length mismatch: expected {}, got {}", expected, actual),
                        Self::VarDataTooLong { field, max_length, actual } => write!(f, "var data too long for field {}: max {}, actual {}", field, max_length, actual),
                        Self::FixedArrayTooLong { field, max_length, actual } => write!(f, "fixed array too long for field {}: max {}, actual {}", field, max_length, actual),
                        Self::InvalidAscii { field } => write!(f, "field '{}': invalid ASCII", field),
                        Self::ValueOutOfRange { field, min, max, actual } => write!(f, "value out of range for field {}: min {}, max {}, actual {}", field, min, max, actual),
                        Self::GroupFull { declared, attempted } => write!(f, "group full: declared count {}, attempted to write {}", declared, attempted),
                        Self::GroupCountMismatch { declared, actual } => write!(f, "group count mismatch: declared {declared}, wrote {actual}"),
                        Self::GroupCountOverflow { maximum, actual } => write!(f, "group count overflow: max {maximum}, actual {actual}"),
                        Self::EncodedLengthOverflow => write!(f, "encoded length computation overflowed"),
                        Self::DomainConversionFailed { field, reason } => write!(f, "domain conversion failed for field {field}: {reason}"),
                        Self::Decode(e) => write!(f, "decode error: {e}"),
                    }
                }
            }

            impl core::error::Error for EncodeError {
                fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
                    match self {
                        Self::Decode(e) => Some(e),
                        _ => None,
                    }
                }
            }

            impl From<DecodeError> for EncodeError {
                #[inline]
                fn from(e: DecodeError) -> Self {
                    Self::Decode(e)
                }
            }

            /// Meta attribute selector (Java `MetaAttribute` parity).
            #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
            pub enum MetaAttribute {
                /// Epoch / start of time (e.g. `"unix"`).
                Epoch,
                /// Time unit applied to the epoch (e.g. `"nanosecond"`).
                TimeUnit,
                /// SBE semantic type / FIX-tag relationship.
                SemanticType,
                /// Field presence: `"required"`, `"optional"`, or `"constant"`.
                Presence,
            }

            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub enum VerifyError {
                /// Buffer shorter than the message header.
                HeaderTooShort,
                /// Wire block length below the minimum readable for this version.
                InvalidBlockLength { expected_min: usize, actual: usize },
                /// Group dimension header for `field` lies past the buffer end.
                GroupDimOutOfBounds { field: &'static str, offset: usize },
                /// Var-data region for `field` lies past the buffer end.
                VarDataOutOfBounds { field: &'static str, offset: usize, length: u64 },
                /// Full message (header + tails) longer than available bytes.
                MessageTooShort { needed: usize, available: usize },
                /// Nested decode error while verifying.
                DecodeError(DecodeError),
            }

            impl core::fmt::Display for VerifyError {
                #[cold]
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    match self {
                        Self::HeaderTooShort => write!(f, "buffer too short to contain message header"),
                        Self::InvalidBlockLength { expected_min, actual } => write!(f, "invalid block length: expected at least {}, actual {}", expected_min, actual),
                        Self::GroupDimOutOfBounds { field, offset } => write!(f, "group dimension header for '{}' out of bounds at offset {}", field, offset),
                        Self::VarDataOutOfBounds { field, offset, length } => write!(f, "var-data for '{}' out of bounds at offset {} with length {}", field, offset, length),
                        Self::MessageTooShort { needed, available } => write!(f, "message too short: needed {} bytes, {} available", needed, available),
                        Self::DecodeError(e) => write!(f, "decode error during verification: {e}"),
                    }
                }
            }

            impl From<DecodeError> for VerifyError {
                #[inline]
                fn from(e: DecodeError) -> Self {
                    VerifyError::DecodeError(e)
                }
            }

            impl core::error::Error for VerifyError {
                fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
                    match self {
                        Self::DecodeError(e) => Some(e),
                        _ => None,
                    }
                }
            }

            /// Convert a wire var-data length without truncation and validate
            /// the complete region with overflow-safe offset arithmetic.
            #[inline]
            pub(crate) fn checked_var_data_bounds(
                field: &'static str,
                offset: usize,
                prefix_size: usize,
                wire_length: u64,
                buffer_length: usize,
            ) -> Result<(usize, usize), DecodeError> {
                let length = usize::try_from(wire_length).map_err(|_| {
                    DecodeError::InvalidVarDataLength {
                        field,
                        length: wire_length,
                        max_length: usize::MAX as u64,
                    }
                })?;
                let data_start = offset.checked_add(prefix_size).ok_or(
                    DecodeError::BufferTooShort {
                        field,
                        needed: usize::MAX,
                        available: buffer_length.saturating_sub(offset),
                    },
                )?;
                let data_end = data_start.checked_add(length).ok_or(
                    DecodeError::BufferTooShort {
                        field,
                        needed: usize::MAX,
                        available: buffer_length.saturating_sub(offset),
                    },
                )?;
                if data_end > buffer_length {
                    return Err(DecodeError::BufferTooShort {
                        field,
                        needed: prefix_size.saturating_add(length),
                        available: buffer_length.saturating_sub(offset),
                    });
                }
                Ok((data_start, data_end))
            }

            #[inline]
            pub(crate) fn checked_header_u16(
                field: &'static str,
                value: u64,
            ) -> Result<u16, DecodeError> {
                u16::try_from(value).map_err(|_| DecodeError::InvalidHeaderValue {
                    field,
                    value,
                    maximum: u16::MAX as u64,
                })
            }

            #[inline]
            pub(crate) fn checked_header_usize(
                field: &'static str,
                value: u64,
            ) -> Result<usize, DecodeError> {
                usize::try_from(value).map_err(|_| DecodeError::InvalidHeaderValue {
                    field,
                    value,
                    maximum: usize::MAX as u64,
                })
            }

            /// Group `numInGroup` must fit both `usize` and the implementation
            /// iteration ceiling (`u32::MAX`). Never truncate a parser-accepted
            /// unsigned dimension into a narrower diagnostic or loop count.
            #[inline]
            pub(crate) fn checked_group_count(
                field: &'static str,
                value: u64,
            ) -> Result<usize, DecodeError> {
                if value > u32::MAX as u64 {
                    return Err(DecodeError::InvalidHeaderValue {
                        field,
                        value,
                        maximum: u32::MAX as u64,
                    });
                }
                checked_header_usize(field, value)
            }

            /// Narrow a group count for `GroupFull` / mismatch diagnostics.
            /// Errors instead of truncating when the count exceeds `u32::MAX`.
            #[inline]
            pub(crate) fn group_diag_count(count: u64) -> Result<u32, EncodeError> {
                u32::try_from(count).map_err(|_| EncodeError::GroupCountOverflow {
                    maximum: u32::MAX,
                    actual: u32::MAX,
                })
            }

            /// Compile-time metadata for a generated SBE message.
            ///
            /// Sealed: the supertrait lives in a private child of the generated
            /// module, so only code generated alongside these types can name
            /// it. Generic framing code can therefore trust that a
            /// `T: SbeMessage` carries this schema's real template id, block
            /// length, schema id, and version.
            #[diagnostic::on_unimplemented(
                message = "`{Self}` is not a generated SBE message type",
                note = "SbeMessage is a sealed trait — only types generated by `ergo_sbe::Generator` can implement it. Import the generated module and use the provided decoder/encoder types directly."
            )]
            pub trait SbeMessage: super::__sbe_message_sealed::Sealed {
                const TEMPLATE_ID: u16;
                const BLOCK_LENGTH: usize;
                const SCHEMA_ID: u16;
                const SCHEMA_VERSION: u16;
            }

            mod private {
                pub trait Sealed {}
            }

            /// How a group decoder was obtained.
            ///
            /// A group reached through its message's tail is **attached**: the
            /// decoder knows the real parent body, so completing it can hand
            /// back the next message stage. A group wrapped standalone is
            /// **detached**: it iterates, random-accesses, and rewinds, but has
            /// no parent to return to, so it exposes no message-stage
            /// completion. The distinction is a zero-sized type parameter —
            /// no runtime field, branch, or allocation.
            pub trait GroupContext: private::Sealed {}

            /// Standalone group: no parent message stage. Default context.
            #[doc(hidden)]
            pub struct Detached(());
            impl private::Sealed for Detached {}
            impl GroupContext for Detached {}

            /// Group reached through a message tail; completion is available.
            ///
            /// There is deliberately no safe public constructor: attachment is
            /// a proof, not a claim a consumer can make.
            #[doc(hidden)]
            pub struct Attached(());
            impl private::Sealed for Attached {}
            impl GroupContext for Attached {}

            // Zero-sized header-state markers (defined per-module to avoid
            // forcing a runtime dependency on ergo_sbe).
            pub trait HeaderState: private::Sealed {}
            pub struct HeaderPresent;
            impl private::Sealed for HeaderPresent {}
            impl HeaderState for HeaderPresent {}
            pub struct HeaderAbsent;
            impl private::Sealed for HeaderAbsent {}
            impl HeaderState for HeaderAbsent {}

            /// Typestate for whether fixed fields have been committed via
            /// `fixed(&FixedFields)`. Tail (group/var-data) methods are only
            /// available on [`FieldsFixed`].
            pub trait FieldsState: private::Sealed {}
            /// Initial encoder stage — fixed fields / `fixed()` / `raw_fixed()` only.
            pub struct FieldsUnfixed;
            impl private::Sealed for FieldsUnfixed {}
            impl FieldsState for FieldsUnfixed {}
            /// After `fixed(&FixedFields)` — ordered group/var-data tails only.
            pub struct FieldsFixed;
            impl private::Sealed for FieldsFixed {}
            impl FieldsState for FieldsFixed {}

            /// Return type for group closures (`add`, `bids`, …).
            /// Closures return `Result<(), EncodeError>`; `?` just works.
            pub type GroupResult = Result<(), EncodeError>;

            pub trait IntoGroupResult {
                fn into_group_result(self) -> GroupResult;
            }
            impl IntoGroupResult for () {
                #[inline]
                fn into_group_result(self) -> GroupResult { Ok(()) }
            }
            impl IntoGroupResult for GroupResult {
                #[inline]
                fn into_group_result(self) -> GroupResult { self }
            }

        }
    };

    syn::parse_str::<syn::File>(&module.to_string())
        .map(|file| prettyplease::unparse(&file))
        .expect("generated SBE runtime must be valid Rust syntax")
}

/// Token appended when a generated identifier collides with a Rust keyword.
/// Set for the duration of a codegen pass via [`with_keyword_append`].
thread_local! {
    static KEYWORD_APPEND: std::cell::RefCell<String> = std::cell::RefCell::new("_".into());
}

/// Run `f` with a custom keyword-append token (Java `sbe.keyword.append.token`).
pub(crate) fn with_keyword_append<R>(token: &str, f: impl FnOnce() -> R) -> R {
    KEYWORD_APPEND.with(|cell| {
        let prev = cell.replace(token.to_string());
        let out = f();
        *cell.borrow_mut() = prev;
        out
    })
}

fn keyword_append_token() -> String {
    KEYWORD_APPEND.with(|c| c.borrow().clone())
}

/// Name of the generated module's private sealing child.
pub(crate) const SEALED_MODULE: &str = "__sbe_message_sealed";

/// Per-schema state that travels explicitly through one generation pass,
/// rather than living in ambient global/thread-local state a re-entrant hook
/// (one that invokes a nested `Generator`) could observe or clobber.
///
/// Currently holds only the sealing path — the module holding the
/// `SbeMessage` sealing trait, as seen from the generated module currently
/// being emitted. A module that emits its own `sbe_rt` owns the sealing
/// module outright, so the path is the bare private child
/// ([`SEALED_MODULE`]). A module importing a shared runtime has to name the
/// owner's copy instead — otherwise its message types could not satisfy the
/// shared `SbeMessage` supertrait at all.
pub(crate) struct GenerationContext {
    pub(crate) sealed_path: syn::Path,
}

/// The private sealing module declaration for a generated module that owns the
/// runtime. `exported` widens it to `pub(super)` so sibling modules generated
/// against a shared runtime can still implement it; a self-contained module
/// keeps it fully private, which is what makes `SbeMessage` unimplementable
/// outside the generated module.
pub(crate) fn generate_sealed_module_src(exported: bool) -> String {
    // Always pub(crate): sibling generated modules in the same crate can
    // still name `super::shared::__sbe_message_sealed`, but external crates
    // cannot implement SbeMessage. The `pub mod` path was a metadata-forgery
    // risk when the shared module itself is public.
    let _ = exported; // kept for API compatibility, always crate-private
    let visibility = "pub(crate) ";
    format!(
        "/// Sealing marker for [`sbe_rt::SbeMessage`]. Private to this generated\n\
         /// module: no consumer can name it, so no consumer can forge message\n\
         /// metadata by implementing `SbeMessage` for its own type.\n\
         {visibility}mod {SEALED_MODULE} {{\n    \
         pub trait Sealed {{}}\n\
         }}\n\n"
    )
}

thread_local! {
    static DEPRECATED_ATTRS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Run `f` with `#[deprecated]` emission enabled (`with_deprecated_attrs()`).
/// When off (default), no `#[deprecated]` is emitted anywhere — avoiding the
/// Rust cascade where deprecating a generated type warns at every `impl` on it.
pub(crate) fn with_deprecated_attrs<R>(enabled: bool, f: impl FnOnce() -> R) -> R {
    DEPRECATED_ATTRS.with(|cell| {
        let prev = cell.get();
        cell.set(enabled);
        let out = f();
        cell.set(prev);
        out
    })
}

fn deprecated_attrs_enabled() -> bool {
    DEPRECATED_ATTRS.with(|c| c.get())
}

/// Rust keywords that cannot be used as bare identifiers.
pub(crate) fn is_rust_keyword(s: &str) -> bool {
    matches!(
        s,
        "as" | "async"
            | "await"
            | "break"
            | "const"
            | "continue"
            | "crate"
            | "dyn"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "abstract"
            | "become"
            | "box"
            | "do"
            | "final"
            | "macro"
            | "override"
            | "priv"
            | "try"
            | "typeof"
            | "unsized"
            | "virtual"
            | "yield"
            | "gen"
            | "union"
    )
}

fn avoid_keyword(mut name: String) -> String {
    // `_` is a reserved identifier in Rust — can't be used as a raw ident (`r#_` also fails).
    // Rename it explicitly so field/getter names don't break compilation.
    if name == "_" {
        return "underscore".to_string();
    }
    if is_rust_keyword(&name) {
        name.push_str(&keyword_append_token());
    }
    name
}

pub(crate) fn to_pascal_case(s: &str) -> String {
    let mut res = String::new();
    let mut capitalize_next = true;
    let mut prev_is_lower = false;
    for c in s.chars() {
        if c == '_' || c == '-' || c == ' ' {
            capitalize_next = true;
            prev_is_lower = false;
        } else if c.is_uppercase() {
            if prev_is_lower {
                capitalize_next = true;
            }
            if capitalize_next {
                res.extend(c.to_uppercase());
                capitalize_next = false;
            } else {
                res.push(c);
            }
            prev_is_lower = false;
        } else {
            if capitalize_next {
                res.extend(c.to_uppercase());
                capitalize_next = false;
            } else {
                res.push(c);
            }
            prev_is_lower = true;
        }
    }
    avoid_keyword(res)
}

/// Names already claimed by enums, sets, and composites in a schema module.
/// Used to disambiguate `{Message}Schema` constant markers.
pub(crate) fn occupied_type_names(elements: &SchemaElements) -> std::collections::HashSet<String> {
    let mut names = std::collections::HashSet::new();
    for e in &elements.enums {
        names.insert(to_pascal_case(&e[0].name));
    }
    for s in &elements.sets {
        names.insert(to_pascal_case(&s[0].name));
    }
    for c in &elements.composites {
        names.insert(to_pascal_case(&c[0].name));
    }
    names
}

/// Marker type holding schema constants for a message (`CarSchema::TEMPLATE_ID`).
///
/// When a composite/enum/set is already named `{Msg}Schema`, use
/// `{Msg}MessageSchema` (then numbered suffixes) so the module still compiles.
pub(crate) fn schema_marker_ident(
    msg_pascal: &str,
    occupied: &std::collections::HashSet<String>,
) -> syn::Ident {
    let primary = format!("{msg_pascal}Schema");
    if !occupied.contains(&primary) {
        return syn::Ident::new(&primary, proc_macro2::Span::call_site());
    }
    let alt = format!("{msg_pascal}MessageSchema");
    if !occupied.contains(&alt) {
        return syn::Ident::new(&alt, proc_macro2::Span::call_site());
    }
    let mut n = 2usize;
    loop {
        let name = format!("{msg_pascal}MessageSchema{n}");
        if !occupied.contains(&name) {
            return syn::Ident::new(&name, proc_macro2::Span::call_site());
        }
        n += 1;
    }
}

pub(crate) fn to_snake_case(s: &str) -> String {
    let mut res = String::new();
    let mut prev_is_lower = false;
    let mut _prev_is_upper = false;
    for c in s.chars() {
        if c == '_' || c == '-' || c == ' ' {
            res.push('_');
            prev_is_lower = false;
            _prev_is_upper = false;
        } else if c.is_uppercase() {
            if prev_is_lower {
                res.push('_');
            }
            res.extend(c.to_lowercase());
            prev_is_lower = false;
            _prev_is_upper = true;
        } else {
            res.push(c);
            prev_is_lower = true;
            _prev_is_upper = false;
        }
    }
    let mut clean = String::new();
    for c in res.chars() {
        if c == '_' && clean.ends_with('_') {
            continue;
        }
        clean.push(c);
    }
    avoid_keyword(clean)
}

pub(crate) fn to_upper_snake_case(s: &str) -> String {
    // Uppercase after snake conversion; re-apply keyword rule on the upper form
    // only if the snake form itself was not already rewritten (keywords are lowercase).
    to_snake_case(s).to_uppercase()
}

pub(crate) fn constant_value_expr(prim: PrimitiveType, val: &str) -> String {
    // Convert dotted valueRef like "TimeUnit.nanosecond" → "TimeUnit::Nanosecond".
    // Only if the prefix is non-numeric — float constants like "1.5" must not
    // be mistaken for enum references.
    if let Some((enum_name, variant)) = val.split_once('.')
        && enum_name.chars().any(|c| !c.is_ascii_digit())
    {
        let enum_ref = format!("{}::{}", to_pascal_case(enum_name), to_pascal_case(variant));
        return match prim {
            PrimitiveType::UInt8 => format!("{enum_ref} as u8"),
            PrimitiveType::UInt16 => format!("{enum_ref} as u16"),
            PrimitiveType::UInt32 => format!("{enum_ref} as u32"),
            PrimitiveType::UInt64 => format!("{enum_ref} as u64"),
            PrimitiveType::Int8 => format!("{enum_ref} as i8"),
            PrimitiveType::Int16 => format!("{enum_ref} as i16"),
            PrimitiveType::Int32 => format!("{enum_ref} as i32"),
            PrimitiveType::Int64 => format!("{enum_ref} as i64"),
            _ => enum_ref,
        };
    }
    match prim {
        // parser validates single-char constants, generator trusts it; add a debug_assert! if parser ever skips validation
        PrimitiveType::Char => format!("b'{}'", val),
        PrimitiveType::Float => {
            format!("{}f32", val)
        }
        PrimitiveType::Double => {
            format!("{}f64", val)
        }
        _ => {
            format!("{}", val)
        }
    }
}

/// Format a `u64` stored value as a valid Rust literal expression for the given type.
pub(crate) fn field_const_value_expr(val: u64, prim: PrimitiveType) -> String {
    match prim {
        PrimitiveType::Char | PrimitiveType::UInt8 => format!("{val}_u8"),
        PrimitiveType::UInt16 => format!("{val}_u16"),
        PrimitiveType::UInt32 => format!("{val}_u32"),
        PrimitiveType::UInt64 => format!("{val}_u64"),
        PrimitiveType::Int8 => format!("{}_i8", val as i8),
        PrimitiveType::Int16 => format!("{}_i16", val as i16),
        PrimitiveType::Int32 => format!("{}_i32", val as i32),
        PrimitiveType::Int64 => format!("{}_i64", val as i64),
        PrimitiveType::Float => format!("f32::from_bits({}u32)", val as u32),
        PrimitiveType::Double => format!("f64::from_bits({val})"),
    }
}

/// Emit field metadata + `*_NULL`/`*_MIN`/`*_MAX` constants (Java field statics parity).
pub(crate) fn emit_field_consts(f: &MessageField) -> proc_macro2::TokenStream {
    let upper_name = to_upper_snake_case(&f.name);
    let snake_name = to_snake_case(&f.name);
    let span = proc_macro2::Span::call_site();
    let mut tokens = proc_macro2::TokenStream::new();

    // Structural metadata (id / sinceVersion / offset / length) — always emitted.
    if let Some(id) = f.id {
        let id_ident = syn::Ident::new(&format!("{upper_name}_ID"), span);
        let id_lit = syn::LitInt::new(&id.to_string(), span);
        tokens.extend(quote::quote! {
            pub const #id_ident: u16 = #id_lit;
        });
    }
    {
        let since_ident = syn::Ident::new(&format!("{upper_name}_SINCE_VERSION"), span);
        let since_lit = syn::LitInt::new(&f.since_version.to_string(), span);
        let off_ident = syn::Ident::new(&format!("{upper_name}_ENCODING_OFFSET"), span);
        let off_lit = syn::LitInt::new(&f.offset.to_string(), span);
        let len_ident = syn::Ident::new(&format!("{upper_name}_ENCODING_LENGTH"), span);
        let enc_len = f.field_type.size();
        let len_lit = syn::LitInt::new(&enc_len.to_string(), span);
        tokens.extend(quote::quote! {
            pub const #since_ident: u16 = #since_lit;
            pub const #off_ident: usize = #off_lit;
            pub const #len_ident: usize = #len_lit;
        });
    }

    // MetaAttribute lookup (epoch / timeUnit / semanticType / presence).
    {
        let meta_fn = syn::Ident::new(&format!("{snake_name}_meta_attribute"), span);
        let presence = match f.presence {
            crate::Presence::Required => "required",
            crate::Presence::Optional => "optional",
            crate::Presence::Constant => "constant",
        };
        let presence_lit = syn::LitStr::new(presence, span);
        let epoch_arm = match f.epoch.as_deref() {
            Some(e) => {
                let lit = syn::LitStr::new(e, span);
                quote::quote! { Some(#lit) }
            }
            None => quote::quote! { None },
        };
        let time_arm = match f.time_unit.as_deref() {
            Some(t) => {
                let lit = syn::LitStr::new(t, span);
                quote::quote! { Some(#lit) }
            }
            None => quote::quote! { None },
        };
        let sem_arm = match f.semantic_type.as_deref() {
            Some(s) => {
                let lit = syn::LitStr::new(s, span);
                quote::quote! { Some(#lit) }
            }
            None => quote::quote! { None },
        };
        tokens.extend(quote::quote! {
            #[inline]
            pub const fn #meta_fn(attr: sbe_rt::MetaAttribute) -> Option<&'static str> {
                match attr {
                    sbe_rt::MetaAttribute::Epoch => #epoch_arm,
                    sbe_rt::MetaAttribute::TimeUnit => #time_arm,
                    sbe_rt::MetaAttribute::SemanticType => #sem_arm,
                    sbe_rt::MetaAttribute::Presence => Some(#presence_lit),
                }
            }
        });
    }

    match &f.field_type {
        FieldType::Primitive(prim, _) => {
            let r_type = rust_type(*prim);
            let r_type_ty: syn::Type = syn::parse_str(r_type).unwrap();
            if let Some(val) = f.null_value {
                let name_ident = syn::Ident::new(&format!("{upper_name}_NULL"), span);
                let expr = field_const_value_expr(val, *prim);
                let expr_parsed: syn::Expr = syn::parse_str(&expr).unwrap();
                tokens.extend(quote::quote! {
                    pub const #name_ident: #r_type_ty = #expr_parsed;
                });
            }
            if let Some(val) = f.min_value {
                let name_ident = syn::Ident::new(&format!("{upper_name}_MIN"), span);
                let expr = field_const_value_expr(val, *prim);
                let expr_parsed: syn::Expr = syn::parse_str(&expr).unwrap();
                tokens.extend(quote::quote! {
                    pub const #name_ident: #r_type_ty = #expr_parsed;
                });
            }
            if let Some(val) = f.max_value {
                let name_ident = syn::Ident::new(&format!("{upper_name}_MAX"), span);
                let expr = field_const_value_expr(val, *prim);
                let expr_parsed: syn::Expr = syn::parse_str(&expr).unwrap();
                tokens.extend(quote::quote! {
                    pub const #name_ident: #r_type_ty = #expr_parsed;
                });
            }
        }
        FieldType::Enum {
            name,
            encoding_type: _,
        } => {
            let target_name = to_pascal_case(name);
            let name_ident = syn::Ident::new(&format!("{upper_name}_NULL"), span);
            let target_ident = syn::Ident::new(&target_name, span);
            tokens.extend(quote::quote! {
                pub const #name_ident: #target_ident = #target_ident::NullVal;
            });
        }
        FieldType::Composite { .. } | FieldType::Set { .. } => {}
    }
    tokens
}

/// Emit the version-aware `min_readable_fixed_extent` body:
/// `let mut m = N; if acting_version >= V { m = M; } … m`.
///
/// Required, non-constant fields that are active at `max_version` must fit.
/// Optional and Constant fields are excluded: constants occupy no wire bytes,
/// and optional getters carry their own block-length guard.
///
/// This is NOT purely a safety bound — it doubles as a frame validity check.
/// Including a since-versioned required field in the extent rejects a
/// malformed frame *before* the decoder is constructed, rather than letting
/// the getter silently return `None`.
pub(crate) fn emit_readable_extent_body(fields: &[MessageField]) -> proc_macro2::TokenStream {
    let span = proc_macro2::Span::call_site();
    let in_extent =
        |f: &&MessageField| f.presence != Presence::Optional && f.presence != Presence::Constant;
    let extent_at = |max_version: u16| -> usize {
        fields
            .iter()
            .filter(in_extent)
            .filter(|f| f.since_version <= max_version)
            .map(|f| f.offset.saturating_add(f.field_type.size()))
            .max()
            .unwrap_or(0)
    };

    let mut versions: Vec<u16> = fields
        .iter()
        .filter(in_extent)
        .map(|f| f.since_version)
        .collect();
    versions.sort_unstable();
    versions.dedup();

    let m0 = extent_at(0);
    let m0_lit = syn::LitInt::new(&m0.to_string(), span);

    // Build one TokenStream per version guard branch so we can splice them
    // into one quote! block.
    let mut version_arms = proc_macro2::TokenStream::new();
    for &v in &versions {
        if v == 0 {
            continue;
        }
        let v_lit = syn::LitInt::new(&v.to_string(), span);
        let m = extent_at(v);
        let m_lit = syn::LitInt::new(&m.to_string(), span);
        version_arms.extend(quote::quote! {
            if acting_version >= #v_lit {
                m = #m_lit;
            }
        });
    }

    quote::quote! {
        let mut m = #m0_lit;
        #version_arms
        m
    }
}

pub(crate) fn find_matching_end(
    tokens: &[Token],
    start: usize,
    begin: Signal,
    end: Signal,
) -> usize {
    let mut depth = 1;
    for j in (start + 1)..tokens.len() {
        if tokens[j].signal == begin {
            depth += 1;
        } else if tokens[j].signal == end {
            depth -= 1;
            if depth == 0 {
                return j;
            }
        }
    }
    tokens.len() - 1
}

pub(crate) fn generate_enum(src: &mut String, tokens: &[Token]) {
    let raw_name = &tokens[0].name;
    let name = to_pascal_case(raw_name);
    let encoding_type = tokens[0]
        .encoding
        .primitive_type
        .unwrap_or(PrimitiveType::UInt8);
    let r_type = rust_type(encoding_type);
    let is_char = encoding_type == PrimitiveType::Char;

    let name_ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
    let r_type_ty: syn::Type = syn::parse_str(&r_type).unwrap();

    struct Variant {
        variant_ident: syn::Ident,
        disc: proc_macro2::TokenStream,
        description: Option<String>,
    }

    let variants: Vec<Variant> = tokens
        .iter()
        .filter(|t| t.signal == Signal::Encoding)
        .filter_map(|t| {
            let val = t.encoding.constant_value.as_ref()?;
            let variant_ident =
                syn::Ident::new(&to_pascal_case(&t.name), proc_macro2::Span::call_site());
            let variant_desc = t.encoding.description.clone();
            let disc: proc_macro2::TokenStream = if is_char {
                let byte = val.as_bytes().first().copied().unwrap_or(0);
                let lit = syn::LitByte::new(byte, proc_macro2::Span::call_site());
                quote::quote! { #lit }
            } else {
                let lit = val
                    .parse::<u64>()
                    .ok()
                    .map(|v| syn::LitInt::new(&v.to_string(), proc_macro2::Span::call_site()))
                    .or_else(|| {
                        val.parse::<i64>().ok().map(|v| {
                            syn::LitInt::new(&v.to_string(), proc_macro2::Span::call_site())
                        })
                    })
                    .unwrap_or_else(|| syn::LitInt::new(val, proc_macro2::Span::call_site()));
                quote::quote! { #lit }
            };
            Some(Variant {
                variant_ident,
                disc,
                description: variant_desc,
            })
        })
        .collect();

    let variant_names: Vec<_> = variants.iter().map(|v| &v.variant_ident).collect();
    let variant_discs: Vec<_> = variants.iter().map(|v| &v.disc).collect();

    let from_raw_arms: Vec<_> = variants
        .iter()
        .map(|v| {
            let disc = &v.disc;
            let vname = &v.variant_ident;
            quote::quote! { #disc => Self::#vname }
        })
        .collect();

    // Detect boolean enum type: name convention, semanticType attribute, or
    // exactly two variants that form a true/false pair (same heuristic as
    // structured_ir::is_bool_value_enum so that auto_bool_domain and trait
    // emission agree).
    let is_bool = tokens[0].name == "BooleanType"
        || tokens[0].encoding.semantic_type.as_deref() == Some("Boolean")
        || (variants.len() == 2 && {
            let names: Vec<String> = variants
                .iter()
                .map(|v| v.variant_ident.to_string())
                .collect();
            crate::structured_ir::is_boolean_value_pair(&names[0], &names[1])
        });

    let (false_ident, true_ident) = if is_bool {
        let f = variants
            .iter()
            .find(|v| v.disc.to_string() == "0")
            .map(|v| v.variant_ident.clone());
        let t = variants
            .iter()
            .find(|v| v.disc.to_string() == "1")
            .map(|v| v.variant_ident.clone());
        (f, t)
    } else {
        (None, None)
    };

    // From<bool> / From<Name> for bool impls for boolean types
    let from_bool_impl = if let (Some(fv), Some(tv)) = (&false_ident, &true_ident) {
        quote::quote! {
            impl From<bool> for #name_ident {
                #[inline]
                fn from(val: bool) -> Self {
                    if val { Self::#tv } else { Self::#fv }
                }
            }

            impl TryFrom<#name_ident> for bool {
                type Error = ();
                #[inline]
                fn try_from(val: #name_ident) -> Result<Self, Self::Error> {
                    val.as_bool().ok_or(())
                }
            }
        }
    } else {
        quote::quote! {}
    };

    // as_bool() merged into the main impl block to avoid a separate impl
    // block that shifts code layout (LTO alignment regression).
    let as_bool_method = if let (Some(fv), Some(tv)) = (&false_ident, &true_ident) {
        quote::quote! {
            /// Returns `Some(true)` / `Some(false)` for the valid boolean
            /// values. Returns `None` for `NullVal` or any unknown raw
            /// discriminant — the SBE boolean wire type is tri-state
            /// (F, T, null). Prefer this (or `TryFrom`) over treating the
            /// raw discriminant as a Rust `bool`.
            #[must_use = "discarding this value is almost always a mistake"]
            #[inline]
            pub const fn as_bool(self) -> Option<bool> {
                match self {
                    Self::#fv => Some(false),
                    Self::#tv => Some(true),
                    _ => None,
                }
            }
        }
    } else {
        quote::quote! {}
    };

    // NullVal discriminant: use the schema's nullValue if set.
    // null_value is stored as u64 — reinterpret for signed encoding types.
    let null_disc: syn::LitInt = tokens[0]
        .encoding
        .null_value
        .map(|nv| {
            let val_str: String = match encoding_type {
                PrimitiveType::Int8 => (nv as i8 as i64).to_string(),
                PrimitiveType::Int16 => (nv as i16 as i64).to_string(),
                PrimitiveType::Int32 => (nv as i32 as i64).to_string(),
                PrimitiveType::Int64 => (nv as i64).to_string(),
                _ => nv.to_string(),
            };
            syn::LitInt::new(&val_str, proc_macro2::Span::call_site())
        })
        .unwrap_or_else(|| {
            let nv: i64 = match encoding_type {
                PrimitiveType::UInt8 => 255,
                PrimitiveType::UInt16 => 65535,
                PrimitiveType::UInt32 => 4_294_967_295_i64,
                PrimitiveType::UInt64 => i64::MAX,
                PrimitiveType::Int8 => -128,
                PrimitiveType::Int16 => -32768,
                PrimitiveType::Int32 => -2_147_483_648,
                PrimitiveType::Int64 => i64::MIN,
                PrimitiveType::Char => 0,
                _ => 255,
            };
            syn::LitInt::new(&nv.to_string(), proc_macro2::Span::call_site())
        });
    let null_disc_ts: proc_macro2::TokenStream = quote::quote! { #null_disc };

    if let Some(ref desc) = tokens[0].encoding.description {
        push_description_doc(src, desc);
    } else {
        src.push_str("/// SBE enum `");
        src.push_str(&tokens[0].name);
        src.push_str("` — wire discriminant ");
        src.push_str(&r_type);
        src.push_str(".\n");
    }

    // Build variant declarations with doc comments
    let variant_decls: Vec<proc_macro2::TokenStream> = variants
        .iter()
        .map(|v| {
            let name = &v.variant_ident;
            let disc = &v.disc;
            if let Some(ref desc) = v.description {
                let doc = doc_attr_tokens(desc);
                quote::quote! {
                    #doc
                    #name = #disc,
                }
            } else {
                quote::quote! { #name = #disc, }
            }
        })
        .collect();

    let tokens = quote::quote! {
        #[repr(#r_type_ty)]
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum #name_ident {
            #(#variant_decls)*
            /// Unknown enum value — the wire discriminant did not match any known variant.
            NullVal = #null_disc_ts,
        }

        impl #name_ident {
            /// Wire discriminant.
            #[must_use = "discarding this value is almost always a mistake"]
            #[inline]
            pub fn raw(self) -> #r_type_ty {
                self as #r_type_ty
            }

            /// Reconstruct from a wire discriminant (`NullVal` for unknown).
            #[inline]
            pub const fn from_raw(val: #r_type_ty) -> Self {
                match val {
                    #(#from_raw_arms,)*
                    _ => Self::NullVal,
                }
            }

            /// Map [`Self::NullVal`] → [`None`], any other variant → [`Some`].
            #[must_use = "discarding this value is almost always a mistake"]
            #[inline]
            pub const fn as_option(self) -> Option<Self> {
                if matches!(self, Self::NullVal) { None } else { Some(self) }
            }

            #as_bool_method
        }

        impl From<#name_ident> for #r_type_ty {
            #[inline]
            fn from(val: #name_ident) -> Self {
                val as #r_type_ty
            }
        }

        impl From<#r_type_ty> for #name_ident {
            #[inline]
            fn from(val: #r_type_ty) -> Self {
                Self::from_raw(val)
            }
        }

        impl core::fmt::Display for #name_ident {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                match self {
                    #(Self::#variant_names => f.write_str(stringify!(#variant_names)),)*
                    Self::NullVal => f.write_str("NullVal"),
                }
            }
        }

        impl core::str::FromStr for #name_ident {
            type Err = ();

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    #(stringify!(#variant_names) => Ok(Self::#variant_names),)*
                    "NullVal" => Ok(Self::NullVal),
                    _ => Err(()),
                }
            }
        }

        #from_bool_impl
    };

    let formatted = syn::parse_str::<syn::File>(&tokens.to_string())
        .map(|file| prettyplease::unparse(&file))
        .unwrap_or_else(|_| tokens.to_string());
    src.push_str(&formatted);
    src.push('\n');
}

pub(crate) fn generate_set(src: &mut String, tokens: &[Token]) {
    let raw_name = &tokens[0].name;
    let name = to_pascal_case(raw_name);
    let encoding_type = tokens[0]
        .encoding
        .primitive_type
        .unwrap_or(PrimitiveType::UInt8);
    let r_type = rust_type(encoding_type);

    let name_ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
    let r_type_ty: syn::Type = syn::parse_str(&r_type).unwrap();

    let mut bits: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut choice_getters: Vec<syn::Ident> = Vec::new();
    let mut choice_setters: Vec<syn::Ident> = Vec::new();
    let mut choice_name_strs: Vec<syn::LitStr> = Vec::new();
    for t in tokens.iter().filter(|t| t.signal == Signal::Encoding) {
        let Some(val) = t.encoding.constant_value.as_ref() else {
            continue;
        };
        let bit_index: u8 = val.parse().unwrap_or(0);
        let snake = to_snake_case(&t.name);
        let is_bit_name = quote::format_ident!("is_{}", snake);
        let set_bit_name = syn::Ident::new(&snake, proc_macro2::Span::call_site());
        let bit_lit = syn::LitInt::new(&bit_index.to_string(), proc_macro2::Span::call_site());
        choice_getters.push(is_bit_name.clone());
        choice_setters.push(set_bit_name.clone());
        choice_name_strs.push(syn::LitStr::new(&t.name, proc_macro2::Span::call_site()));
        bits.push(quote::quote! {
            #[must_use = "discarding this value is almost always a mistake"]
            #[inline]
            pub const fn #is_bit_name(self) -> bool {
                (self.0 & (1 << #bit_lit)) != 0
            }

            #[inline]
            pub fn #set_bit_name(&mut self, val: bool) -> &mut Self {
                if val {
                    self.0 |= 1 << #bit_lit;
                } else {
                    self.0 &= !(1 << #bit_lit);
                }
                self
            }
        });
    }

    // Emit set doc from the type's XML description.
    if let Some(ref desc) = tokens[0].encoding.description {
        push_description_doc(src, desc);
    } else {
        src.push_str("/// SBE bitset `");
        src.push_str(&tokens[0].name);
        src.push_str("` — wire type ");
        src.push_str(&r_type);
        src.push_str(".\n");
    }

    let tokens = quote::quote! {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
        #[repr(transparent)]
        pub struct #name_ident(pub #r_type_ty);

        impl #name_ident {
            #[must_use = "discarding this value is almost always a mistake"]
            #[inline]
            pub const fn raw(self) -> #r_type_ty {
                self.0
            }

            #[inline]
            pub const fn default() -> Self {
                Self(0)
            }

            #(#bits)*
        }

        impl From<#r_type_ty> for #name_ident {
            #[inline]
            fn from(val: #r_type_ty) -> Self {
                Self(val)
            }
        }

        impl From<#name_ident> for #r_type_ty {
            #[inline]
            fn from(val: #name_ident) -> Self {
                val.0
            }
        }

        impl core::fmt::Display for #name_ident {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                let mut first = true;
                #(
                    if self.#choice_getters() {
                        if !first {
                            f.write_str("|")?;
                        }
                        f.write_str(#choice_name_strs)?;
                        first = false;
                    }
                )*
                Ok(())
            }
        }

        impl core::str::FromStr for #name_ident {
            type Err = ();

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                let mut v = Self::default();
                if s.is_empty() {
                    return Ok(v);
                }
                for part in s.split('|') {
                    let part = part.trim();
                    let mut matched = false;
                    #(
                        if part == #choice_name_strs {
                            v.#choice_setters(true);
                            matched = true;
                        }
                    )*
                    if !matched {
                        return Err(());
                    }
                }
                Ok(v)
            }
        }
    };

    let formatted = syn::parse_str::<syn::File>(&tokens.to_string())
        .map(|file| prettyplease::unparse(&file))
        .unwrap_or_else(|_| tokens.to_string());
    src.push_str(&formatted);
    src.push('\n');
}

/// Emit a composite as a **wire image** plus flyweight decoder.
///
/// Layout contract (do not “simplify” to `#[repr(C)]` native fields):
/// - `#[repr(transparent)] pub struct Name(pub [u8; N])` — value == on-wire bytes
/// - getters/setters use `from_{le,be}_bytes` / `to_{le,be}_bytes` at schema offsets
/// - flyweight `NameDecoder { buf, offset }` for zero-copy reads from a message buffer
/// - `const _: () = assert!(size_of::<Name>() == N)`
///
/// On little-endian hosts, `from_le_bytes` is effectively a plain load; this is
/// the safe equivalent of “overlay a struct on the buffer” without padding or
/// unaligned UB. See README “Composite layout & little-endian”.
/// Null-sentinel comparison for a `presence="optional"` composite member
/// (e.g. a Decimal composite's `mantissa`). Mirrors the top-level optional
/// primitive field check in `message_decoder.rs`: any IEEE NaN is null for
/// float/double (matches sbe-tool's `is_nan()`), otherwise wire equality
/// against the schema `nullValue` (default 0, matching top-level optionals).
fn composite_member_null_check(
    prim: PrimitiveType,
    null_value: Option<u64>,
) -> proc_macro2::TokenStream {
    if prim == PrimitiveType::Float || prim == PrimitiveType::Double {
        quote::quote! { val.is_nan() }
    } else {
        let null_val = null_value.unwrap_or(0);
        let null_lit = syn::LitInt::new(&format!("{null_val}_u64"), proc_macro2::Span::call_site());
        let r_type_ty: syn::Type = syn::parse_str(rust_type(prim)).unwrap();
        quote::quote! { val == #null_lit as #r_type_ty }
    }
}

pub(crate) fn generate_composite(src: &mut String, tokens: &[Token], byte_order: ByteOrder) {
    let raw_name = &tokens[0].name;
    let name = to_pascal_case(raw_name);
    let size = tokens[0].encoding.offset.unwrap_or(0);

    let members = parse_composite_members(tokens);

    let has_float = members.iter().any(|m| {
        matches!(
            &m.member_type,
            MemberType::Primitive {
                prim: PrimitiveType::Float | PrimitiveType::Double,
                ..
            }
        )
    });

    let name_ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
    let size_lit = syn::LitInt::new(&size.to_string(), proc_macro2::Span::call_site());

    let derives = if has_float {
        quote::quote! { Clone, Copy, PartialEq, PartialOrd }
    } else {
        quote::quote! { Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash }
    };

    let order_suffix = match byte_order {
        ByteOrder::LittleEndian => "le",
        ByteOrder::BigEndian => "be",
    };
    let from_method = syn::Ident::new(
        &format!("from_{order_suffix}_bytes"),
        proc_macro2::Span::call_site(),
    );
    let to_method = syn::Ident::new(
        &format!("to_{order_suffix}_bytes"),
        proc_macro2::Span::call_site(),
    );

    let mut getters = proc_macro2::TokenStream::new();
    let mut ctor_params = Vec::new();
    let mut ctor_body = proc_macro2::TokenStream::new();

    for m in &members {
        let field_name = to_snake_case(&m.name);
        let field_ident = syn::Ident::new(&field_name, proc_macro2::Span::call_site());
        let offset_lit = syn::LitInt::new(&m.offset.to_string(), proc_macro2::Span::call_site());

        if let Some(ref desc) = m.description {
            getters.extend(doc_attr_tokens(desc));
        }

        match &m.member_type {
            MemberType::Primitive {
                prim,
                length,
                presence,
                constant_value,
                null_value,
            } => {
                let r_type_str = rust_type(*prim);
                let r_type_ty: syn::Type = syn::parse_str(r_type_str).unwrap();
                let prim_size = prim.size();
                let prim_size_lit =
                    syn::LitInt::new(&prim_size.to_string(), proc_macro2::Span::call_site());

                if *presence == Presence::Constant {
                    if let Some(val) = constant_value {
                        if *prim == PrimitiveType::Char && val.len() > 1 {
                            let val_lit = syn::LitStr::new(val, proc_macro2::Span::call_site());
                            getters.extend(quote::quote! {
                                #[must_use = "discarding this value is almost always a mistake"]
                                #[inline]
                                pub const fn #field_ident(&self) -> &'static str {
                                    #val_lit
                                }
                            });
                        } else {
                            let expr_str = constant_value_expr(*prim, val);
                            let expr: syn::Expr = syn::parse_str(&expr_str).unwrap();
                            getters.extend(quote::quote! {
                                #[must_use = "discarding this value is almost always a mistake"]
                                #[inline]
                                pub const fn #field_ident(&self) -> #r_type_ty {
                                    #expr
                                }
                            });
                        }
                    }
                    continue; // no ctor param for constants
                }

                if let Some(len) = length {
                    let len_lit =
                        syn::LitInt::new(&len.to_string(), proc_macro2::Span::call_site());
                    let array_ty: syn::Type =
                        syn::parse_str(&format!("[{}; {}]", r_type_str, len)).unwrap();
                    ctor_params.push(quote::quote! { #field_ident: #array_ty });

                    if *len > 0 {
                        getters.extend(quote::quote! {
                            #[must_use = "discarding this value is almost always a mistake"]
                            #[inline]
                            pub fn #field_ident(&self) -> [#r_type_ty; #len_lit] {
                                let mut res = [0 as #r_type_ty; #len_lit];
                                let mut idx = 0;
                                while idx < #len_lit {
                                    let offset = #offset_lit + idx * #prim_size_lit;
                                    res[idx] = #r_type_ty::#from_method(
                                        read_bytes::<#prim_size_lit>(&self.0, offset)
                                    );
                                    idx += 1;
                                }
                                res
                            }
                        });

                        ctor_body.extend(quote::quote! {
                            let mut idx = 0;
                            while idx < #len_lit {
                                let val_bytes = #field_ident[idx].#to_method();
                                write_bytes::<#prim_size_lit>(&mut bytes, #offset_lit + idx * #prim_size_lit, &val_bytes);
                                idx += 1;
                            }
                        });
                    } else {
                        // zero-length array: return empty array immediately
                        let zero_ty: syn::Type =
                            syn::parse_str(&format!("[{}; 0]", r_type_str)).unwrap();
                        getters.extend(quote::quote! {
                            #[must_use = "discarding this value is almost always a mistake"]
                            #[inline]
                            pub fn #field_ident(&self) -> #zero_ty {
                                []
                            }
                        });
                    }
                } else {
                    ctor_params.push(quote::quote! { #field_ident: #r_type_ty });

                    if *presence == Presence::Optional {
                        let null_check = composite_member_null_check(*prim, *null_value);
                        getters.extend(quote::quote! {
                            #[must_use = "discarding this value is almost always a mistake"]
                            #[inline]
                            pub fn #field_ident(&self) -> Option<#r_type_ty> {
                                let val = #r_type_ty::#from_method(read_bytes::<#prim_size_lit>(&self.0, #offset_lit));
                                if #null_check { None } else { Some(val) }
                            }
                        });
                    } else {
                        getters.extend(quote::quote! {
                            #[must_use = "discarding this value is almost always a mistake"]
                            #[inline]
                            pub fn #field_ident(&self) -> #r_type_ty {
                                #r_type_ty::#from_method(read_bytes::<#prim_size_lit>(&self.0, #offset_lit))
                            }
                        });
                    }

                    ctor_body.extend(quote::quote! {
                        let val_bytes = #field_ident.#to_method();
                        write_bytes::<#prim_size_lit>(&mut bytes, #offset_lit, &val_bytes);
                    });
                }
            }
            MemberType::Composite {
                name: comp_name,
                size: comp_size,
            } => {
                let target_name = to_pascal_case(comp_name);
                let target_ident = syn::Ident::new(&target_name, proc_macro2::Span::call_site());
                let comp_size_lit =
                    syn::LitInt::new(&comp_size.to_string(), proc_macro2::Span::call_site());

                ctor_params.push(quote::quote! { #field_ident: #target_ident });

                getters.extend(quote::quote! {
                    #[must_use = "discarding this value is almost always a mistake"]
                    #[inline]
                    pub fn #field_ident(&self) -> #target_ident {
                        #target_ident(read_bytes::<#comp_size_lit>(&self.0, #offset_lit))
                    }
                });

                ctor_body.extend(quote::quote! {
                    write_bytes::<#comp_size_lit>(&mut bytes, #offset_lit, &#field_ident.0);
                });
            }
            MemberType::Enum {
                name: enum_name,
                encoding_type,
            } => {
                let target_name = to_pascal_case(enum_name);
                let target_ident = syn::Ident::new(&target_name, proc_macro2::Span::call_site());
                let r_type = rust_type(*encoding_type);
                let r_type_ty: syn::Type = syn::parse_str(&r_type).unwrap();
                let prim_size = encoding_type.size();
                let prim_size_lit =
                    syn::LitInt::new(&prim_size.to_string(), proc_macro2::Span::call_site());
                let raw_ident = syn::Ident::new(
                    &format!("raw_{}", field_name),
                    proc_macro2::Span::call_site(),
                );

                ctor_params.push(quote::quote! { #field_ident: #target_ident });

                getters.extend(quote::quote! {
                    #[must_use = "discarding this value is almost always a mistake"]
                    #[inline]
                    pub fn #field_ident(&self) -> #target_ident {
                        #target_ident::from_raw(#r_type_ty::#from_method(
                            read_bytes::<#prim_size_lit>(&self.0, #offset_lit)
                        ))
                    }
                    /// Raw wire discriminant — bypasses enum mapping.
                    #[must_use = "discarding this value is almost always a mistake"]
                    #[inline]
                    pub fn #raw_ident(&self) -> #r_type_ty {
                        #r_type_ty::#from_method(
                            read_bytes::<#prim_size_lit>(&self.0, #offset_lit)
                        )
                    }
                });

                ctor_body.extend(quote::quote! {
                    let val_bytes = (#field_ident as #r_type_ty).#to_method();
                    write_bytes::<#prim_size_lit>(&mut bytes, #offset_lit, &val_bytes);
                });
            }
            MemberType::Set {
                name: set_name,
                encoding_type,
            } => {
                let target_name = to_pascal_case(set_name);
                let target_ident = syn::Ident::new(&target_name, proc_macro2::Span::call_site());
                let r_type = rust_type(*encoding_type);
                let r_type_ty: syn::Type = syn::parse_str(&r_type).unwrap();
                let prim_size = encoding_type.size();
                let prim_size_lit =
                    syn::LitInt::new(&prim_size.to_string(), proc_macro2::Span::call_site());

                ctor_params.push(quote::quote! { #field_ident: #target_ident });

                getters.extend(quote::quote! {
                    #[must_use = "discarding this value is almost always a mistake"]
                    #[inline]
                    pub fn #field_ident(&self) -> #target_ident {
                        #target_ident(#r_type_ty::#from_method(
                            read_bytes::<#prim_size_lit>(&self.0, #offset_lit)
                        ))
                    }
                });

                ctor_body.extend(quote::quote! {
                    let val_bytes = #field_ident.0.#to_method();
                    write_bytes::<#prim_size_lit>(&mut bytes, #offset_lit, &val_bytes);
                });
            }
        }
    }

    if let Some(ref desc) = tokens[0].encoding.description {
        push_description_doc(src, desc);
    } else {
        src.push_str("/// SBE composite `");
        src.push_str(&tokens[0].name);
        src.push_str("` — ");
        src.push_str(&size_lit.to_string());
        src.push_str(" byte wire image.\n");
    }

    let debug_impl = {
        let s = proc_macro2::Span::call_site();
        let mut fields_fmt = proc_macro2::TokenStream::new();
        for m in &members {
            let fn_name = syn::Ident::new(&to_snake_case(&m.name), s);
            let name_str = m.name.as_str();
            fields_fmt.extend(quote::quote! {
                .field(#name_str, &self.#fn_name())
            });
        }
        quote::quote! {
            impl core::fmt::Debug for #name_ident {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.debug_struct(stringify!(#name_ident))
                        #fields_fmt
                        .finish()
                }
            }
        }
    };

    let ts = quote::quote! {
        #[derive(#derives)]
        #[repr(transparent)]
        pub struct #name_ident(pub [u8; #size_lit]);

        #debug_impl

        impl #name_ident {
            #getters

            #[inline]
            pub fn new(#(#ctor_params),*) -> Self {
                let mut bytes = [0u8; #size_lit];
                #ctor_body
                Self(bytes)
            }
        }

        // Compile-time proof that the Rust struct matches the wire size.
        const _: () = assert!(core::mem::size_of::<#name_ident>() == #size_lit);
    };

    src.push_str(&ts.to_string());

    // MessageHeader convenience: peek methods + ENCODED_LENGTH so callers
    // do not duplicate the schema-declared header layout for dispatch.
    if raw_name == "messageHeader" {
        let hs_lit = size_lit.clone();
        let extras = quote::quote! {
            /// Canonical wire size of the SBE message header.
            pub const MESSAGE_HEADER_ENCODED_LENGTH: usize = #hs_lit;

            /// Parsed `(template_id, schema_id)` from [`MessageHeader::peek_header`].
            /// Named fields prevent silent transposition of the two `u16` values.
            #[derive(Clone, Copy, Debug, PartialEq, Eq)]
            pub struct PeekedHeader {
                pub template_id: u16,
                pub schema_id: u16,
            }

            impl #name_ident {
                /// Read the header fields from a buffer without constructing a
                /// full `MessageHeader`. Returns `None` when the buffer is
                /// shorter than the header.
                #[must_use = "the peeked header identity is unused; ignoring it skips dispatch"]
                #[inline]
                pub fn peek_header(data: &[u8]) -> Option<PeekedHeader> {
                    if data.len() < #hs_lit {
                        return None;
                    }
                    let mut hdr = [0u8; #hs_lit];
                    hdr.copy_from_slice(&data[..#hs_lit]);
                    let this = Self(hdr);
                    let template_id =
                        u16::try_from(this.template_id() as u64).ok()?;
                    let schema_id =
                        u16::try_from(this.schema_id() as u64).ok()?;
                    Some(PeekedHeader { template_id, schema_id })
                }

                /// Read `template_id` from a frame without constructing a full
                /// `MessageHeader`. Returns `None` when the buffer is shorter
                /// than the header. For correct multi-schema dispatch,
                /// prefer [`Self::peek_header`] which also returns `schema_id`.
                #[must_use = "the peeked template id is unused; ignoring it skips dispatch"]
                #[inline]
                pub fn peek_template_id(data: &[u8]) -> Option<u16> {
                    if data.len() < #hs_lit {
                        return None;
                    }
                    let mut hdr = [0u8; #hs_lit];
                    hdr.copy_from_slice(&data[..#hs_lit]);
                    u16::try_from(Self(hdr).template_id() as u64).ok()
                }

                /// Validate `schema_id` and return `template_id`. Returns
                /// `None` when the buffer is too short or the schema doesn't
                /// match. Use this for correct multi-schema dispatch.
                #[must_use = "the schema-matched template id is unused; ignoring it skips dispatch"]
                #[inline]
                pub fn peek_for_schema(data: &[u8], expected_schema_id: u16) -> Option<u16> {
                    let header = Self::peek_header(data)?;
                    if header.schema_id == expected_schema_id { Some(header.template_id) } else { None }
                }
            }
        };
        src.push_str(&extras.to_string());
    }
    src.push('\n');

    let mut decoder_getters = proc_macro2::TokenStream::new();
    for m in &members {
        let field_name = to_snake_case(&m.name);
        let field_ident = syn::Ident::new(&field_name, proc_macro2::Span::call_site());
        let offset_lit = syn::LitInt::new(&m.offset.to_string(), proc_macro2::Span::call_site());

        if let Some(ref desc) = m.description {
            getters.extend(doc_attr_tokens(desc));
        }

        match &m.member_type {
            MemberType::Primitive {
                prim,
                length,
                presence,
                constant_value,
                null_value,
            } => {
                let r_type_str = rust_type(*prim);
                let r_type_ty: syn::Type = syn::parse_str(r_type_str).unwrap();
                let prim_size = prim.size();
                let prim_size_lit =
                    syn::LitInt::new(&prim_size.to_string(), proc_macro2::Span::call_site());

                if *presence == Presence::Constant {
                    if let Some(val) = constant_value {
                        if *prim == PrimitiveType::Char && val.len() > 1 {
                            let val_lit = syn::LitStr::new(val, proc_macro2::Span::call_site());
                            decoder_getters.extend(quote::quote! {
                                #[must_use = "discarding this value is almost always a mistake"]
                                #[inline]
                                pub const fn #field_ident(&self) -> &'static str {
                                    #val_lit
                                }
                            });
                        } else {
                            let expr_str = constant_value_expr(*prim, val);
                            let expr: syn::Expr = syn::parse_str(&expr_str).unwrap();
                            decoder_getters.extend(quote::quote! {
                                #[must_use = "discarding this value is almost always a mistake"]
                                #[inline]
                                pub const fn #field_ident(&self) -> #r_type_ty {
                                    #expr
                                }
                            });
                        }
                    }
                    continue;
                }

                if let Some(len) = length {
                    let len_lit =
                        syn::LitInt::new(&len.to_string(), proc_macro2::Span::call_site());
                    if *len > 0 {
                        decoder_getters.extend(quote::quote! {
                            #[must_use = "discarding this value is almost always a mistake"]
                            #[inline]
                            pub fn #field_ident(&self) -> [#r_type_ty; #len_lit] {
                                let mut res = [0 as #r_type_ty; #len_lit];
                                let mut idx = 0;
                                while idx < #len_lit {
                                    res[idx] = #r_type_ty::#from_method(
                                        unsafe { read_bytes_unchecked::<#prim_size_lit>(self.buf, self.offset + #offset_lit + idx * #prim_size_lit) }
                                    );
                                    idx += 1;
                                }
                                res
                            }
                        });
                    } else {
                        let zero_ty: syn::Type =
                            syn::parse_str(&format!("[{}; 0]", r_type_str)).unwrap();
                        decoder_getters.extend(quote::quote! {
                            #[must_use = "discarding this value is almost always a mistake"]
                            #[inline]
                            pub fn #field_ident(&self) -> #zero_ty {
                                []
                            }
                        });
                    }
                } else if *presence == Presence::Optional {
                    let null_check = composite_member_null_check(*prim, *null_value);
                    decoder_getters.extend(quote::quote! {
                        #[must_use = "discarding this value is almost always a mistake"]
                        #[inline]
                        pub fn #field_ident(&self) -> Option<#r_type_ty> {
                            let val = #r_type_ty::#from_method(unsafe { read_bytes_unchecked::<#prim_size_lit>(self.buf, self.offset + #offset_lit) });
                            if #null_check { None } else { Some(val) }
                        }
                    });
                } else {
                    decoder_getters.extend(quote::quote! {
                        #[must_use = "discarding this value is almost always a mistake"]
                        #[inline]
                        pub fn #field_ident(&self) -> #r_type_ty {
                            #r_type_ty::#from_method(unsafe { read_bytes_unchecked::<#prim_size_lit>(self.buf, self.offset + #offset_lit) })
                        }
                    });
                }
            }
            MemberType::Composite {
                name: comp_name,
                size: comp_size,
            } => {
                let target_name = to_pascal_case(comp_name);
                let target_ident = syn::Ident::new(&target_name, proc_macro2::Span::call_site());
                let comp_size_lit =
                    syn::LitInt::new(&comp_size.to_string(), proc_macro2::Span::call_site());

                decoder_getters.extend(quote::quote! {
                    #[must_use = "discarding this value is almost always a mistake"]
                    #[inline]
                    pub fn #field_ident(&self) -> #target_ident {
                        #target_ident(unsafe { read_bytes_unchecked::<#comp_size_lit>(self.buf, self.offset + #offset_lit) })
                    }
                });
            }
            MemberType::Enum {
                name: enum_name,
                encoding_type,
            } => {
                let target_name = to_pascal_case(enum_name);
                let target_ident = syn::Ident::new(&target_name, proc_macro2::Span::call_site());
                let r_type = rust_type(*encoding_type);
                let r_type_ty: syn::Type = syn::parse_str(&r_type).unwrap();
                let prim_size = encoding_type.size();
                let prim_size_lit =
                    syn::LitInt::new(&prim_size.to_string(), proc_macro2::Span::call_site());

                decoder_getters.extend(quote::quote! {
                    #[must_use = "discarding this value is almost always a mistake"]
                    #[inline]
                    pub fn #field_ident(&self) -> #target_ident {
                        #target_ident::from_raw(#r_type_ty::#from_method(unsafe { read_bytes_unchecked::<#prim_size_lit>(self.buf, self.offset + #offset_lit) }))
                    }
                });
            }
            MemberType::Set {
                name: set_name,
                encoding_type,
            } => {
                let target_name = to_pascal_case(set_name);
                let target_ident = syn::Ident::new(&target_name, proc_macro2::Span::call_site());
                let r_type = rust_type(*encoding_type);
                let r_type_ty: syn::Type = syn::parse_str(&r_type).unwrap();
                let prim_size = encoding_type.size();
                let prim_size_lit =
                    syn::LitInt::new(&prim_size.to_string(), proc_macro2::Span::call_site());

                decoder_getters.extend(quote::quote! {
                    #[must_use = "discarding this value is almost always a mistake"]
                    #[inline]
                    pub fn #field_ident(&self) -> #target_ident {
                        #target_ident(#r_type_ty::#from_method(unsafe { read_bytes_unchecked::<#prim_size_lit>(self.buf, self.offset + #offset_lit) }))
                    }
                });
            }
        }
    }

    let decoder_name = syn::Ident::new(&format!("{}Decoder", name), proc_macro2::Span::call_site());
    src.push_str("/// Flyweight decoder for the `");
    src.push_str(&tokens[0].name);
    src.push_str("` composite.\n");
    let decoder_ts = quote::quote! {
        #[derive(Clone, Copy)]
        pub struct #decoder_name<'a> {
            pub(crate) buf: &'a [u8],
            /// Byte offset of the composite body within `self.buf`.
            pub(crate) offset: usize,
        }

        impl<'a> #decoder_name<'a> {
            #decoder_getters
        }
    };
    src.push_str(&decoder_ts.to_string());
    src.push('\n');
}

/// Core generator for consuming tail stages, shared by message-level and
/// entry-level tails. Emits non-`Copy` stage structs plus `into_*`, `finish`,
/// and `skip_remaining`. Does not remove random-access `&self` accessors.
///
/// `initial_ident` is the existing decoder (e.g. `CarDecoder`, `BidsEntryDecoder`);
/// `stage_prefix` is its string form, used to name the `After*`/`Complete` stages.
/// `header_size` is the message header size for messages (0 for entries).
pub(crate) fn generate_prelude(
    src: &mut String,
    elements: &SchemaElements,
    messages: &[MessageStructure],
    schema_id: u16,
    schema_version: u16,
    enable_dispatch: bool,
) {
    writeln!(src, "pub const SCHEMA_ID: u16 = {schema_id};").unwrap();
    writeln!(src, "pub const SCHEMA_VERSION: u16 = {schema_version};").unwrap();

    let mut gen_types: Vec<String> = Vec::new();

    for ct in &elements.composites {
        let name = to_pascal_case(&ct[0].name);
        gen_types.push(name.clone());
        gen_types.push(format!("{name}Decoder"));
    }

    for et in &elements.enums {
        gen_types.push(to_pascal_case(&et[0].name));
    }

    for st in &elements.sets {
        gen_types.push(to_pascal_case(&st[0].name));
    }

    for msg in messages {
        gen_types.push(format!("{}Decoder", to_pascal_case(&msg.name)));
        gen_types.push(format!("{}Encoder", to_pascal_case(&msg.name)));
    }

    // sbe_rt types (exported from super::sbe_rt)
    src.push_str("pub mod prelude {\n");
    src.push_str(
        "    pub use super::sbe_rt::{DecodeError, EncodeError, VerifyError, MetaAttribute, SbeMessage};\n",
    );

    // Module-level types (exported from super)
    src.push_str("    pub use super::{\n");
    if enable_dispatch {
        for ty in &[
            "AnyMessage",
            "DecodedFrame",
            "FrameCursor",
            "FramingPolicy",
            "MessageVisitor",
        ] {
            writeln!(src, "        {ty},").unwrap();
        }
    }
    // Generated types (composites, enums, sets, messages)
    for ty in &gen_types {
        writeln!(src, "        {ty},").unwrap();
    }
    src.push_str("    };\n");
    src.push_str("}\n\n");
}

pub(crate) fn generate_schema_id_from_header(
    src: &mut String,
    elements: &SchemaElements,
    header_type: &str,
    byte_order: ByteOrder,
) {
    let order_suffix = match byte_order {
        ByteOrder::LittleEndian => "le",
        ByteOrder::BigEndian => "be",
    };

    let schema_id = elements
        .composites
        .iter()
        .find(|c| c[0].name == header_type)
        .and_then(|comp| {
            parse_composite_members(comp)
                .into_iter()
                .find(|m| m.name.to_lowercase().contains("schemaid"))
                .map(|member| {
                    let (primitive, presence, constant_value) = match member.member_type {
                        MemberType::Primitive {
                            prim,
                            presence,
                            constant_value,
                            ..
                        } => (prim, presence, constant_value),
                        _ => unreachable!("validated schemaId must be primitive"),
                    };
                    let header_size = comp[0].encoding.offset.unwrap_or(0);
                    (
                        member.offset,
                        primitive,
                        presence,
                        constant_value,
                        header_size,
                    )
                })
        });
    let Some((
        schema_id_offset,
        schema_id_primitive,
        schema_id_presence,
        schema_id_constant,
        header_size,
    )) = schema_id
    else {
        src.push_str(
            "/// Read `schemaId` from a message header at the start of `buf`.\n\
             /// Returns [`None`] when the header composite has no `schemaId` field\n\
             /// or `buf` is shorter than the header.\n\
             #[must_use = \"the header schema id is unused; ignoring it skips dispatch\"]\n\
             #[inline]\npub const fn schema_id_from_header(_buf: &[u8]) -> Option<u16> { None }\n",
        );
        return;
    };

    if schema_id_presence == Presence::Constant {
        let Some(value) = schema_id_constant else {
            src.push_str(
                "/// Read `schemaId` from a message header at the start of `buf`.\n\
                 /// Returns [`None`] when `schemaId` is constant but has no value,\n\
                 /// or `buf` is shorter than the header.\n\
                 #[must_use = \"the header schema id is unused; ignoring it skips dispatch\"]\n\
                 #[inline]\npub const fn schema_id_from_header(_buf: &[u8]) -> Option<u16> { None }\n",
            );
            return;
        };
        let value_expr = constant_value_expr(schema_id_primitive, &value);
        let value_expr: syn::Expr = syn::parse_str(&value_expr)
            .expect("validated constant schemaId must be a Rust expression");
        let header_size =
            syn::LitInt::new(&header_size.to_string(), proc_macro2::Span::call_site());
        let ts = quote::quote! {
            /// Read `schemaId` from a standard 8-byte SBE message header at
            /// the start of `buf`. Returns [`None`] if the buffer is shorter
            /// than the header.
            #[must_use = "the header schema id is unused; ignoring it skips dispatch"]
            #[inline]
            pub fn schema_id_from_header(buf: &[u8]) -> Option<u16> {
                if buf.len() < #header_size {
                    return None;
                }
                u16::try_from((#value_expr) as u64).ok()
            }
        };
        src.push_str(&ts.to_string());
        src.push('\n');
        return;
    }

    let sid = syn::Index::from(schema_id_offset);
    let sid_size = syn::LitInt::new(
        &schema_id_primitive.size().to_string(),
        proc_macro2::Span::call_site(),
    );
    let sid_type = syn::Ident::new(
        rust_type(schema_id_primitive),
        proc_macro2::Span::call_site(),
    );
    let order_fn = syn::Ident::new(
        &format!("from_{order_suffix}_bytes"),
        proc_macro2::Span::call_site(),
    );
    let ts = quote::quote! {
        /// Read `schemaId` from a message header at the start of `buf`.
        /// Returns [`None`] if `buf` is shorter than the header field.
        #[must_use = "the header schema id is unused; ignoring it skips dispatch"]
        #[inline]
        pub fn schema_id_from_header(buf: &[u8]) -> Option<u16> {
            if buf.len() < #sid + #sid_size {
                return None;
            }
            let bytes = read_bytes::<#sid_size>(buf, #sid);
            let value = #sid_type::#order_fn(bytes) as u64;
            u16::try_from(value).ok()
        }
    };
    src.push_str(&ts.to_string());
    src.push('\n');
}

pub(crate) fn generate_any_message(
    messages: &[MessageStructure],
    elements: &SchemaElements,
    schema_id: u16,
    header_type: &str,
    schema_name: &str,
    message_markers: &[(String, String)],
) -> proc_macro2::TokenStream {
    let header_size = elements
        .composites
        .iter()
        .find(|c| c[0].name == header_type)
        .and_then(|c| c[0].encoding.offset)
        .unwrap_or(8);

    let (header_bl, header_ti, header_si, header_vr, header_si_constant) = {
        let mut bl = "block_length".to_string();
        let mut ti = "template_id".to_string();
        let mut si = "schema_id".to_string();
        let mut vr = "version".to_string();
        let mut si_constant = false;
        if let Some(comp) = elements
            .composites
            .iter()
            .find(|c| c[0].name == header_type)
        {
            let members = parse_composite_members(comp);
            for m in members {
                let lower = m.name.to_lowercase();
                let is_constant = matches!(
                    m.member_type,
                    MemberType::Primitive {
                        presence: Presence::Constant,
                        ..
                    }
                );
                if lower.contains("blocklength") {
                    bl = to_snake_case(&m.name);
                } else if lower.contains("templateid") {
                    ti = to_snake_case(&m.name);
                } else if lower.contains("schemaid") {
                    si = to_snake_case(&m.name);
                    si_constant = is_constant;
                } else if lower.contains("version") {
                    vr = to_snake_case(&m.name);
                }
            }
        }
        (bl, ti, si, vr, si_constant)
    };

    let span = proc_macro2::Span::call_site();
    let header_type_ident = syn::Ident::new(&to_pascal_case(header_type), span);
    let header_size_lit = syn::LitInt::new(&header_size.to_string(), span);
    let schema_id_lit = syn::LitInt::new(&schema_id.to_string(), span);
    let bl_ident = syn::Ident::new(&header_bl, span);
    let ti_ident = syn::Ident::new(&header_ti, span);
    let si_ident = syn::Ident::new(&header_si, span);
    let vr_ident = syn::Ident::new(&header_vr, span);
    let schema_id_validation = if header_si_constant {
        quote::quote! {}
    } else {
        quote::quote! {
            if schema_id != #schema_id_lit {
                return Err(sbe_rt::DecodeError::WrongSchema {
                    expected: #schema_id_lit,
                    actual: schema_id,
                    expected_name: #schema_name,
                });
            }
        }
    };

    let mut out = proc_macro2::TokenStream::new();

    {
        let mut enum_variants = proc_macro2::TokenStream::new();
        for m in messages {
            let name = quote::format_ident!("{}", to_pascal_case(&m.name));
            let decoder = quote::format_ident!("{}Decoder", to_pascal_case(&m.name));
            enum_variants.extend(quote::quote! {
                #name(#decoder<'a>),
            });
        }
        out.extend(quote::quote! {
            /// Tagged union of every message type in the schema — decode once,
            /// then `match` to access the typed decoder.
            #[non_exhaustive]
            pub enum AnyMessage<'a> {
                #enum_variants
                Unknown {
                    header: #header_type_ident,
                    /// The complete frame: schema-declared message header
                    /// followed by the unparsed body. Not the body alone.
                    frame: &'a [u8],
                },
            }
        });
    }

    out.extend(quote::quote! {
        /// One decoded message with its buffer range and length.
        pub struct DecodedFrame<'a> {
            pub message: AnyMessage<'a>,
            pub range: core::ops::Range<usize>,
            pub len: usize,
        }
    });

    out.extend(quote::quote! {
        /// How frames are delimited in a stream: length-prefixed or fixed-size.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum FramingPolicy {
            LengthPrefixU32Le,
            LengthPrefixU16Le,
            Fixed(usize),
        }
    });

    out.extend(quote::quote! {
        /// Iterator that yields [`DecodedFrame`]s from a byte buffer according
        /// to a [`FramingPolicy`].
        pub struct FrameCursor<'a> {
            buf: &'a [u8],
            offset: usize,
            framing: FramingPolicy,
        }

        impl<'a> FrameCursor<'a> {
            #[inline]
            pub const fn new(buf: &'a [u8], framing: FramingPolicy) -> Self {
                Self { buf, offset: 0, framing }
            }
        }

        impl<'a> core::iter::FusedIterator for FrameCursor<'a> {}

        impl<'a> Iterator for FrameCursor<'a> {
            type Item = Result<DecodedFrame<'a>, sbe_rt::DecodeError>;

            /// Fused after the first error.
            ///
            /// A framing error means the boundary between this frame and the
            /// next is no longer known, so every later offset would be a guess.
            /// The cursor moves to the terminal boundary instead: one `Err`,
            /// then permanent `None`. Re-polling a broken stream used to return
            /// the same error forever.
            #[inline]
            fn next(&mut self) -> Option<Self::Item> {
                if self.offset >= self.buf.len() {
                    return None;
                }
                // Any error below is terminal; parking `offset` at the end is what
                // fuses the iterator without adding a success-path state field.
                let terminal = self.buf.len();
                let (header_len, frame_len) = match self.framing {
                    FramingPolicy::LengthPrefixU32Le => {
                        if 4 > self.buf.len().saturating_sub(self.offset) {
                            let available = self.buf.len().saturating_sub(self.offset);
                            self.offset = terminal;
                            return Some(Err(sbe_rt::DecodeError::BufferTooShort {
                                field: "length prefix",
                                needed: 4,
                                available,
                            }));
                        }
                        let bytes: [u8; 4] = read_bytes::<4>(self.buf, self.offset);
                        let len = u32::from_le_bytes(bytes) as usize;
                        (4, len)
                    }
                    FramingPolicy::LengthPrefixU16Le => {
                        if 2 > self.buf.len().saturating_sub(self.offset) {
                            let available = self.buf.len().saturating_sub(self.offset);
                            self.offset = terminal;
                            return Some(Err(sbe_rt::DecodeError::BufferTooShort {
                                field: "length prefix",
                                needed: 2,
                                available,
                            }));
                        }
                        let bytes: [u8; 2] = read_bytes::<2>(self.buf, self.offset);
                        let len = u16::from_le_bytes(bytes) as usize;
                        (2, len)
                    }
                    FramingPolicy::Fixed(len) => (0, len),
                };

                let available = self.buf.len().saturating_sub(self.offset);
                let frame_start = match self.offset.checked_add(header_len) {
                    Some(value) => value,
                    None => {
                        self.offset = terminal;
                        return Some(Err(sbe_rt::DecodeError::BufferTooShort {
                            field: "frame bounds",
                            needed: usize::MAX,
                            available,
                        }));
                    }
                };
                let frame_end = match frame_start.checked_add(frame_len) {
                    Some(value) => value,
                    None => {
                        self.offset = terminal;
                        return Some(Err(sbe_rt::DecodeError::BufferTooShort {
                            field: "frame bounds",
                            needed: usize::MAX,
                            available,
                        }));
                    }
                };
                if frame_end > self.buf.len() {
                    self.offset = terminal;
                    return Some(Err(sbe_rt::DecodeError::BufferTooShort {
                        field: "frame bounds",
                        needed: header_len.saturating_add(frame_len),
                        available,
                    }));
                }
                match AnyMessage::decode_frame(self.buf, frame_start, frame_len) {
                    Ok(frame) => {
                        self.offset = frame_end;
                        Some(Ok(frame))
                    }
                    Err(e) => {
                        self.offset = terminal;
                        Some(Err(e))
                    }
                }
            }
        }
    });

    {
        let marker_by_msg: std::collections::HashMap<&str, &str> = message_markers
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        let mut decode_arms = proc_macro2::TokenStream::new();
        let mut decode_arms_unchecked = proc_macro2::TokenStream::new();
        for m in messages {
            let name = quote::format_ident!("{}", to_pascal_case(&m.name));
            let decoder = quote::format_ident!("{}Decoder", to_pascal_case(&m.name));
            let schema = syn::Ident::new(
                marker_by_msg[to_pascal_case(&m.name).as_str()],
                proc_macro2::Span::call_site(),
            );
            decode_arms.extend(quote::quote! {
                #schema::TEMPLATE_ID => {
                    // try_wrap enforces version-aware fixed extent.
                    Ok(Self::#name(#decoder::try_wrap(buf, offset, block_length, version)?))
                }
            });
            decode_arms_unchecked.extend(quote::quote! {
                #schema::TEMPLATE_ID => {
                    Ok(Self::#name(unsafe { #decoder::wrap_unchecked(buf, offset, block_length, version) }))
                }
            });
        }

        out.extend(quote::quote! {
            impl<'a> AnyMessage<'a> {
                /// Dispatch a framed message with header + version-aware fixed-extent checks.
                #[inline]
                pub fn try_decode(buf: &'a [u8], offset: usize) -> Result<Self, sbe_rt::DecodeError> {
                    if #header_size_lit > buf.len().saturating_sub(offset) {
                        return Err(sbe_rt::DecodeError::BufferTooShort {
                            field: "message header",
                            needed: #header_size_lit,
                            available: buf.len().saturating_sub(offset),
                        });
                    }
                    let header_bytes = read_bytes::<#header_size_lit>(buf, offset);
                    let header = #header_type_ident(header_bytes);
                    let template_id = sbe_rt::checked_header_u16(
                        "templateId",
                        header.#ti_ident() as u64,
                    )?;
                    let schema_id = sbe_rt::checked_header_u16(
                        "schemaId",
                        header.#si_ident() as u64,
                    )?;
                    let version = sbe_rt::checked_header_u16(
                        "version",
                        header.#vr_ident() as u64,
                    )?;
                    let block_length = sbe_rt::checked_header_usize(
                        "blockLength",
                        header.#bl_ident() as u64,
                    )?;

                    #schema_id_validation

                    match template_id {
                        #decode_arms
                        _ => Err(sbe_rt::DecodeError::UnknownTemplateLength { template_id }),
                    }
                }

                /// Trusted multi-template dispatch. Same checks as
                /// [`Self::try_decode`]; prefer `try_decode` at untrusted
                /// boundaries. Dynamic tails remain checked on consume.
                #[inline]
                pub fn decode(
                    buf: &'a [u8],
                    offset: usize,
                ) -> Result<Self, sbe_rt::DecodeError> {
                    Self::try_decode(buf, offset)
                }
            }
        });
    }

    {
        let marker_by_msg: std::collections::HashMap<&str, &str> = message_markers
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        let mut decode_frame_arms = proc_macro2::TokenStream::new();
        for m in messages {
            let name = quote::format_ident!("{}", to_pascal_case(&m.name));
            let decoder = quote::format_ident!("{}Decoder", to_pascal_case(&m.name));
            let schema = syn::Ident::new(
                marker_by_msg[to_pascal_case(&m.name).as_str()],
                proc_macro2::Span::call_site(),
            );
            let field_name = &m.name;
            decode_frame_arms.extend(quote::quote! {
                #schema::TEMPLATE_ID => {
                    let frame_end = offset.checked_add(frame_len).ok_or(
                        sbe_rt::DecodeError::BufferTooShort {
                            field: #field_name,
                            needed: frame_len,
                            available: buf.len().saturating_sub(offset),
                        }
                    )?;
                    if frame_end > buf.len() {
                        return Err(sbe_rt::DecodeError::BufferTooShort {
                            field: #field_name,
                            needed: frame_len,
                            available: buf.len().saturating_sub(offset),
                        });
                    }
                    let decoder = #decoder::try_decode(&buf[..frame_end], offset)?;
                    Ok(DecodedFrame {
                        message: Self::#name(decoder),
                        range: offset .. frame_end,
                        len: frame_len,
                    })
                }
            });
        }

        out.extend(quote::quote! {
            impl<'a> AnyMessage<'a> {
                /// Decode one externally framed message.
                ///
                /// `frame_len` is header-inclusive: it is the size of the whole
                /// SBE frame starting at `offset` (message header plus body),
                /// not a body-only length. The declared range
                /// `offset .. offset + frame_len` must fit in `buf`, and
                /// `frame_len` must be at least the schema message-header size,
                /// **before** any header field is read. A shorter declared
                /// length returns [`sbe_rt::DecodeError::BufferTooShort`] with
                /// `field: "message header"` so one frame cannot source
                /// identity bytes from the next framing unit.
                #[inline]
                pub fn decode_frame(buf: &'a [u8], offset: usize, frame_len: usize) -> Result<DecodedFrame<'a>, sbe_rt::DecodeError> {
                    let available = buf.len().saturating_sub(offset);
                    let frame_end = match offset.checked_add(frame_len) {
                        Some(end) => end,
                        None => {
                            return Err(sbe_rt::DecodeError::BufferTooShort {
                                field: "message header",
                                needed: #header_size_lit,
                                available,
                            });
                        }
                    };
                    if frame_end > buf.len() {
                        return Err(sbe_rt::DecodeError::BufferTooShort {
                            field: "message header",
                            needed: frame_len,
                            available,
                        });
                    }
                    if frame_len < #header_size_lit {
                        return Err(sbe_rt::DecodeError::BufferTooShort {
                            field: "message header",
                            needed: #header_size_lit,
                            available: frame_len,
                        });
                    }
                    let frame = &buf[offset..frame_end];
                    let header_bytes: [u8; #header_size_lit] = read_bytes::<#header_size_lit>(frame, 0);
                    let header = #header_type_ident(header_bytes);
                    let template_id = sbe_rt::checked_header_u16(
                        "templateId",
                        header.#ti_ident() as u64,
                    )?;
                    let schema_id = sbe_rt::checked_header_u16(
                        "schemaId",
                        header.#si_ident() as u64,
                    )?;
                    let version = sbe_rt::checked_header_u16(
                        "version",
                        header.#vr_ident() as u64,
                    )?;
                    let block_length = sbe_rt::checked_header_usize(
                        "blockLength",
                        header.#bl_ident() as u64,
                    )?;
                    let body_offset = offset + #header_size_lit;

                    #schema_id_validation

                    match template_id {
                        #decode_frame_arms
                        _ => {
                            Ok(DecodedFrame {
                                message: Self::Unknown {
                                    header,
                                    frame,
                                },
                                range: offset .. frame_end,
                                len: frame_len,
                            })
                        }
                    }
                }
            }
        });
    }

    {
        let mut encoded_arms = proc_macro2::TokenStream::new();
        for m in messages {
            let name = quote::format_ident!("{}", to_pascal_case(&m.name));
            encoded_arms.extend(quote::quote! {
                Self::#name(d) => d.encoded_length_with_header(),
            });
        }

        out.extend(quote::quote! {
            impl<'a> AnyMessage<'a> {
                /// Header-inclusive encoded length of this variant.
                /// [`Self::Unknown`] reports the matched frame length.
                #[inline]
                pub fn encoded_length_with_header(&self) -> Result<usize, sbe_rt::DecodeError> {
                    match self {
                        #encoded_arms
                        Self::Unknown { frame, .. } => Ok(frame.len()),
                    }
                }
            }
        });
    }

    {
        let mut as_bytes_arms = proc_macro2::TokenStream::new();
        for m in messages {
            let name = quote::format_ident!("{}", to_pascal_case(&m.name));
            // Header-inclusive view for known variants — matches Unknown's full frame
            // and historical AnyMessage::as_bytes semantics.
            as_bytes_arms.extend(quote::quote! {
                Self::#name(d) => d.as_bytes_with_header(),
            });
        }

        out.extend(quote::quote! {
            impl<'a> AnyMessage<'a> {
                /// Complete SBE frame (message header + body) — for
                /// [`Self::Unknown`] this is the same header-plus-body range
                /// the cursor matched.
                #[inline]
                pub fn as_bytes(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
                    match self {
                        #as_bytes_arms
                        Self::Unknown { frame, .. } => Ok(frame),
                    }
                }
            }
        });
    }

    {
        let mut encode_arms = proc_macro2::TokenStream::new();
        for m in messages {
            let name = quote::format_ident!("{}", to_pascal_case(&m.name));
            encode_arms.extend(quote::quote! {
                Self::#name(d) => {
                    let len = d.encoded_length_with_header()?;
                    // `len` is header-inclusive; copy the full frame, not body-only.
                    if len > buf.len() {
                        return Err(sbe_rt::EncodeError::BufferTooShort {
                                field: "AnyMessage::encode",
                                needed: len,
                            available: buf.len(),
                        });
                    }
                    let bytes = d.as_bytes_with_header()?;
                    buf[..len].copy_from_slice(bytes);
                    Ok(len)
                }
            });
        }

        out.extend(quote::quote! {
            impl<'a> AnyMessage<'a> {
                /// Copy this message's header-inclusive frame into `buf`.
                /// Unknown templates copy the matched frame bytes.
                #[inline]
                pub fn encode(&self, buf: &mut [u8]) -> Result<usize, sbe_rt::EncodeError> {
                    match self {
                        #encode_arms
                        Self::Unknown { frame, .. } => {
                            if frame.len() > buf.len() {
                                return Err(sbe_rt::EncodeError::BufferTooShort {
                                    field: "AnyMessage::encode",
                                    needed: frame.len(),
                                    available: buf.len(),
                                });
                            }
                            buf[..frame.len()].copy_from_slice(frame);
                            Ok(frame.len())
                        }
                    }
                }
            }
        });
    }

    {
        let mut visitor_methods = Vec::new();
        let mut visit_arms = Vec::new();
        for m in messages {
            let name_pascal = to_pascal_case(&m.name);
            let name_snake = to_snake_case(&m.name);
            let method_name = syn::Ident::new(
                &format!("visit_{name_snake}"),
                proc_macro2::Span::call_site(),
            );
            let decoder_ty: syn::Type =
                syn::parse_str(&format!("{name_pascal}Decoder<'_>")).unwrap();
            let variant = syn::Ident::new(&name_pascal, proc_macro2::Span::call_site());
            visitor_methods.push(quote::quote! {
                fn #method_name(&mut self, decoder: &#decoder_ty) -> Self::Output;
            });
            visit_arms.push(quote::quote! {
                Self::#variant(d) => visitor.#method_name(d),
            });
        }

        out.extend(quote::quote! {
            pub trait MessageVisitor {
                type Output;

                #(#visitor_methods)*

                /// Called for unknown template IDs (not in this schema).
                ///
                /// `header` is the parsed schema-declared MessageHeader.
                /// `frame` is the complete frame — message header followed by
                /// the unparsed body — not the body alone. Must be implemented;
                /// there is no panicking default, because an unknown template
                /// is application policy rather than a crash.
                fn visit_unknown(
                    &mut self,
                    header: &#header_type_ident,
                    frame: &[u8],
                ) -> Self::Output;
            }

            impl<'a> AnyMessage<'a> {
                #[inline]
                pub fn visit<V: MessageVisitor>(&self, visitor: &mut V) -> V::Output {
                    match self {
                        #(#visit_arms)*
                        Self::Unknown { header, frame } => visitor.visit_unknown(header, frame),
                    }
                }
            }
        });
    }

    out
}

/// Compute a canonical wire fingerprint for a token slice representing an
/// enum, set, or composite.
///
/// Includes schema [`ByteOrder`] so little-endian and big-endian declarations
/// of the same type name cannot be treated as interchangeable shared types.
/// Also compares offsets, lengths, presence, constants, null/min/max,
/// discriminants, and related encoding fields. Returns a deterministic string
/// suitable for equality comparison only (not a cryptographic hash).
pub(crate) fn canonical_token_fingerprint(
    tokens: &[crate::ir::Token],
    byte_order: crate::ir::ByteOrder,
) -> String {
    use std::fmt::Write;
    let mut fp = String::new();
    let _ = write!(fp, "bo{:?}:", byte_order);
    for t in tokens {
        let _ = write!(fp, "{}:{:?}:", t.name, t.signal);
        let e = &t.encoding;
        let _ = write!(fp, "sv{}:", e.since_version);
        if let Some(p) = e.primitive_type {
            let _ = write!(fp, "p{:?}:", p);
        }
        if let Some(o) = e.offset {
            let _ = write!(fp, "o{}:", o);
        }
        if let Some(l) = e.length {
            let _ = write!(fp, "l{}:", l);
        }
        if e.presence != crate::ir::Presence::Required {
            let _ = write!(fp, "pr{:?}:", e.presence);
        }
        if let Some(ref cv) = e.constant_value {
            let _ = write!(fp, "cv{}:", cv);
        }
        if let Some(nv) = e.null_value {
            let _ = write!(fp, "nv{}:", nv);
        }
        if let Some(mi) = e.min_value {
            let _ = write!(fp, "mi{}:", mi);
        }
        if let Some(mx) = e.max_value {
            let _ = write!(fp, "mx{}:", mx);
        }
        if let Some(ref st) = e.semantic_type {
            let _ = write!(fp, "st{}:", st);
        }
        if let Some(version) = e.deprecated {
            let _ = write!(fp, "dep{version}:");
        }
    }
    fp
}

/// XML-comment prose (e.g. cluster protocol codecs). Rustdoc treats 4-space
/// indented blocks as Rust doctests, which then fail `cargo test --doc`.
/// Fence multi-line content as `text` so it stays documentation only.
pub(crate) fn sanitize_description_for_doc(desc: &str) -> String {
    let desc = desc.trim_end_matches(['\r', '\n']);
    if !desc.contains('\n') {
        // A schema description is external XML data, not trusted rustdoc
        // markdown/HTML. Rendered `#[doc]` text interprets a bare `<...>` as
        // an HTML tag, so prose like `Option<u32>` fails
        // `rustdoc::invalid_html_tags` under `-D warnings`. Escape it to
        // literal entities so it always renders as written. The multiline
        // branch below doesn't need this: its ``` fence already makes the
        // content literal, and escaping there would show `&amp;` verbatim.
        return escape_doc_html(desc);
    }
    let fence = if desc.contains("```") { "````" } else { "```" };
    format!("{fence}text\n{desc}\n{fence}")
}

/// Escape characters markdown/rustdoc would otherwise interpret as HTML.
fn escape_doc_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// `#[doc = "..."]` token for a schema description (doctest-safe).
pub(crate) fn doc_attr_tokens(desc: &str) -> proc_macro2::TokenStream {
    let lit = syn::LitStr::new(
        &sanitize_description_for_doc(desc),
        proc_macro2::Span::call_site(),
    );
    quote::quote! { #[doc = #lit] }
}

/// One `#[doc]` attribute per line of `text`.
///
/// A single `#[doc]` holding embedded newlines is rendered by `prettyplease`
/// as a `/** … */` block whose continuation lines carry the item's
/// indentation. Markdown reads a 4-space-indented line after a blank line as
/// an indented **code block**, so rustdoc then tries to compile the prose as a
/// doctest and fails. Emitting one attribute per line renders as `///`, which
/// cannot form an indented code block.
pub(crate) fn doc_lines_tokens(text: &str) -> proc_macro2::TokenStream {
    let mut out = proc_macro2::TokenStream::new();
    for line in text.split('\n') {
        let lit = syn::LitStr::new(line, proc_macro2::Span::call_site());
        out.extend(quote::quote! { #[doc = #lit] });
    }
    out
}

/// `#[deprecated(note = "SBE schema deprecated since version N")]` when the
/// item is schema-deprecated AND `with_deprecated_attrs()` is active.
pub(crate) fn deprecated_attr_tokens(deprecated_since: Option<u16>) -> proc_macro2::TokenStream {
    match (deprecated_since, deprecated_attrs_enabled()) {
        (Some(version), true) => {
            let note = format!("SBE schema deprecated since version {version}");
            quote::quote! { #[deprecated(note = #note)] }
        }
        _ => quote::quote! {},
    }
}

/// Append `///` rustdoc lines for a schema description (doctest-safe).
///
/// Single-line style is `///Text` (no forced space). Multi-line content is
/// first fenced as `text` by [`sanitize_description_for_doc`].
pub(crate) fn push_description_doc(src: &mut String, desc: &str) {
    for line in sanitize_description_for_doc(desc).lines() {
        src.push_str("///");
        src.push_str(line);
        src.push('\n');
    }
}

/// Compute a deterministic 64-bit hash of the schema identity.
///
/// Uses FNV-1a over `package` bytes, `id` (LE), and `version` (LE).
/// This is a simple compile-time-expressible hash for schema identity
/// verification — not a cryptographic hash.
pub(crate) fn compute_schema_hash(package: &str, id: u16, version: u16) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    let mut hash = FNV_OFFSET;
    for &b in package.as_bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    for &b in &id.to_le_bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    for &b in &version.to_le_bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Compute SHA-256 hash of the canonical schema IR.
pub(crate) fn compute_schema_sha256(ir: &Ir) -> [u8; 32] {
    let canonical = canonical_schema_bytes(ir);
    let mut hasher = Sha256::new();
    hasher.update(&canonical);
    let result = hasher.finalize();
    result.into()
}

/// Serialize the schema IR to a canonical byte sequence for hashing.
/// The output is deterministic for the same IR content.
pub(crate) fn canonical_schema_bytes(ir: &Ir) -> Vec<u8> {
    let mut buf = Vec::new();

    // Schema identity
    extend_str(&mut buf, &ir.package);
    buf.extend_from_slice(&ir.id.to_le_bytes());
    buf.extend_from_slice(&ir.version.to_le_bytes());
    buf.push(match ir.byte_order {
        ByteOrder::LittleEndian => 0,
        ByteOrder::BigEndian => 1,
    });
    extend_opt_str(&mut buf, ir.description.as_deref());
    extend_opt_str(&mut buf, ir.semantic_version.as_deref());
    extend_str(&mut buf, &ir.header_type);

    for token in &ir.tokens {
        buf.push(token.signal as u8);
        extend_str(&mut buf, &token.name);
        match token.id {
            Some(id) => {
                buf.push(1);
                buf.extend_from_slice(&id.to_le_bytes());
            }
            None => buf.push(0),
        }

        match token.encoding.primitive_type {
            Some(pt) => {
                buf.push(1);
                buf.push(pt as u8);
            }
            None => buf.push(0),
        }
        buf.push(token.encoding.presence as u8);
        buf.extend_from_slice(&token.encoding.since_version.to_le_bytes());
        match token.encoding.null_value {
            Some(nv) => {
                buf.push(1);
                buf.extend_from_slice(&nv.to_le_bytes());
            }
            None => buf.push(0),
        }
        extend_opt_str(&mut buf, token.encoding.character_encoding.as_deref());
        extend_opt_str(&mut buf, token.encoding.semantic_type.as_deref());
        match token.encoding.min_value {
            Some(mv) => {
                buf.push(1);
                buf.extend_from_slice(&mv.to_le_bytes());
            }
            None => buf.push(0),
        }
        match token.encoding.max_value {
            Some(mv) => {
                buf.push(1);
                buf.extend_from_slice(&mv.to_le_bytes());
            }
            None => buf.push(0),
        }
        extend_opt_str(&mut buf, token.encoding.description.as_deref());
        extend_opt_str(&mut buf, token.encoding.constant_value.as_deref());
        match token.encoding.length {
            Some(len) => {
                buf.push(1);
                buf.extend_from_slice(&(len as u64).to_le_bytes());
            }
            None => buf.push(0),
        }
        match token.encoding.offset {
            Some(off) => {
                buf.push(1);
                buf.extend_from_slice(&(off as u64).to_le_bytes());
            }
            None => buf.push(0),
        }
    }

    buf
}

/// Append a null-terminated string to the canonical hash input.
pub(crate) fn extend_str(buf: &mut Vec<u8>, s: &str) {
    buf.extend_from_slice(s.as_bytes());
    buf.push(0);
}

/// Append an optional null-terminated string (presence-tagged).
pub(crate) fn extend_opt_str(buf: &mut Vec<u8>, s: Option<&str>) {
    match s {
        Some(s) => {
            buf.push(1);
            extend_str(buf, s);
        }
        None => buf.push(0),
    }
}

/// Generate a `field_meta` module for a message, exposing field metadata
/// as a compile-time constant slice.
///
/// Emits:
/// Emitted module layout:
///
/// ```
/// pub mod car_field_meta {
///     pub struct FieldInfo {
///         pub name: &'static str,
///         pub id: u16,
///         pub offset: usize,
///         pub since_version: u16,
///         pub field_type: &'static str,
///     }
///     pub const FIELDS: &[FieldInfo] = &[
///         FieldInfo { name: "serialNumber", id: 1, offset: 0, since_version: 0, field_type: "uint64" },
///         FieldInfo { name: "modelYear", id: 2, offset: 8, since_version: 0, field_type: "uint16" },
///     ];
/// }
/// ```
pub(crate) fn generate_message_field_meta(src: &mut String, msg: &MessageStructure) {
    let mod_name = syn::Ident::new(
        &format!("{}_field_meta", to_snake_case(&msg.name)),
        proc_macro2::Span::call_site(),
    );

    let fields: Vec<proc_macro2::TokenStream> = msg
        .fields
        .iter()
        .map(|f| {
            let name_lit = syn::LitStr::new(&f.name, proc_macro2::Span::call_site());
            let id = f.id.unwrap_or(0);
            let id_lit = syn::LitInt::new(&id.to_string(), proc_macro2::Span::call_site());
            let offset_lit =
                syn::LitInt::new(&f.offset.to_string(), proc_macro2::Span::call_site());
            let sv_lit = syn::LitInt::new(
                &f.since_version.to_string(),
                proc_macro2::Span::call_site(),
            );
            let field_type_str = match &f.field_type {
                FieldType::Primitive(prim, _) => rust_type(*prim).to_string(),
                FieldType::Composite { name, .. } => to_pascal_case(name),
                FieldType::Enum { name, .. } => to_pascal_case(name),
                FieldType::Set { name, .. } => to_pascal_case(name),
            };
            let field_type_lit =
                syn::LitStr::new(&field_type_str, proc_macro2::Span::call_site());
            let presence_str = match f.presence {
                Presence::Required => "required",
                Presence::Optional => "optional",
                Presence::Constant => "constant",
            };
            let presence_lit =
                syn::LitStr::new(presence_str, proc_macro2::Span::call_site());
            let null_val = f.null_value.map(|v| {
                let s = v.to_string();
                let lit = syn::LitStr::new(&s, proc_macro2::Span::call_site());
                quote::quote! { Some(#lit) }
            }).unwrap_or(quote::quote! { None });
            let sem_type = f
                .semantic_type
                .as_deref()
                .map(|v| {
                    let lit = syn::LitStr::new(v, proc_macro2::Span::call_site());
                    quote::quote! { Some(#lit) }
                })
                .unwrap_or(quote::quote! { None });
            let desc = f
                .description
                .as_deref()
                .map(|v| {
                    let lit = syn::LitStr::new(v, proc_macro2::Span::call_site());
                    quote::quote! { Some(#lit) }
                })
                .unwrap_or(quote::quote! { None });

            quote::quote! {
                FieldInfo { name: #name_lit, id: #id_lit, offset: #offset_lit, since_version: #sv_lit, field_type: #field_type_lit, presence: #presence_lit, null_value: #null_val, semantic_type: #sem_type, description: #desc },
            }
        })
        .collect();

    let tokens = quote::quote! {
        pub mod #mod_name {
            pub struct FieldInfo {
                pub name: &'static str,
                pub id: u16,
                pub offset: usize,
                pub since_version: u16,
                pub field_type: &'static str,
                pub presence: &'static str,
                pub null_value: Option<&'static str>,
                pub semantic_type: Option<&'static str>,
                pub description: Option<&'static str>,
            }
            pub const FIELDS: &[FieldInfo] = &[
                #(#fields)*
            ];
        }
    };

    let formatted = syn::parse_str::<syn::File>(&tokens.to_string())
        .map(|file| prettyplease::unparse(&file))
        .unwrap_or_else(|_| tokens.to_string());
    src.push_str(&formatted);
    src.push('\n');
}
