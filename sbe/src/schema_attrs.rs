//! Canonical SBE attribute allow-lists.
//!
//! Shared by the always-on parser checks ([`crate::parse`]) and the opt-in
//! XSD-shaped validator ([`crate::validate_against_sbe_xsd`]) so the two
//! cannot drift apart; a divergence would make one reject schemas the other
//! accepts.
//!
//! Each list is the union of the published SBE XSD grammar with the
//! attributes real-world schemas carry. **Neither source alone is complete:**
//! the XSD does not declare `constantValue`, `length`,
//! `nullValue`, or `characterEncoding`, all of which sbe-tool accepts and
//! checked-in schemas use; and no schema corpus exercises every attribute
//! the grammar allows. A list derived from either source on its own rejects
//! valid input.

/// `<messageSchema>` root element.
pub const MESSAGE_SCHEMA: &[&str] = &[
    "package",
    "id",
    "version",
    "semanticVersion",
    "description",
    "byteOrder",
    "headerType",
];

/// `<message>`.
pub const MESSAGE: &[&str] = &[
    "name",
    "id",
    "description",
    "blockLength",
    "semanticType",
    "sinceVersion",
    "deprecated",
];

/// `<field>` and `<data>`.
///
/// One list because the SBE XSD types both elements as `sbe:fieldType`; a
/// narrower hand-written list for `<data>` would drop `epoch`, `offset`,
/// `timeUnit`, and `valueRef`.
pub const FIELD_LIKE: &[&str] = &[
    "name",
    "id",
    "type",
    "description",
    "offset",
    "length",
    "presence",
    "valueRef",
    "constantValue",
    "nullValue",
    "minValue",
    "maxValue",
    "characterEncoding",
    "semanticType",
    "sinceVersion",
    "deprecated",
    "epoch",
    "timeUnit",
];

/// `<enum>`.
///
/// `nullValue` is a schema-declared wire sentinel (sbe-tool accepts it and
/// codegen emits it). The published XSD omits it; the validator still must
/// accept it so XSD-validated parsing matches plain `parse`.
pub const ENUM: &[&str] = &[
    "name",
    "encodingType",
    "description",
    "sinceVersion",
    "deprecated",
    "semanticType",
    "nullValue",
];

/// `<group>`.
pub const GROUP: &[&str] = &[
    "name",
    "id",
    "description",
    "dimensionType",
    "blockLength",
    "semanticType",
    "sinceVersion",
    "deprecated",
];

/// Attributes of the type-level elements (`<type>`, `<composite>`, `<enum>`,
/// `<set>`, `<validValue>`, `<choice>`, `<ref>`) not listed above.
const TYPE_LEVEL: &[&str] = &[
    "primitiveType",
    "length",
    "characterEncoding",
    "nullValue",
    "minValue",
    "maxValue",
    "encodingType",
    "offset",
    "valueRef",
    "presence",
    "epoch",
    "timeUnit",
    "type",
];

/// Whether `name` is an attribute the SBE grammar defines on any element.
/// An unprefixed attribute that is not is an extension mark.
pub fn is_grammar_attr(name: &str) -> bool {
    [MESSAGE_SCHEMA, MESSAGE, FIELD_LIKE, ENUM, GROUP, TYPE_LEVEL]
        .iter()
        .any(|list| list.contains(&name))
}

/// The grammar attribute `name` is a likely misspelling of: within one edit
/// per four characters (at least one), case-insensitive. Names shorter
/// than four characters are never matched, so short custom marks pass.
pub fn near_grammar_attr(name: &str) -> Option<&'static str> {
    if name.len() < 4 {
        return None;
    }
    let lower = name.to_ascii_lowercase();
    [MESSAGE_SCHEMA, MESSAGE, FIELD_LIKE, ENUM, GROUP, TYPE_LEVEL]
        .iter()
        .flat_map(|list| list.iter())
        .copied()
        .filter(|g| g.len() >= 4)
        .find(|g| edit_distance(&lower, &g.to_ascii_lowercase()) <= (g.len() / 4).max(1))
}

/// Optimal string alignment distance: insertions, deletions, substitutions
/// and adjacent transpositions each cost one.
fn edit_distance(a: &str, b: &str) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let mut d = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in d[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut best = (d[i - 1][j] + 1)
                .min(d[i][j - 1] + 1)
                .min(d[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                best = best.min(d[i - 2][j - 2] + 1);
            }
            d[i][j] = best;
        }
    }
    d[a.len()][b.len()]
}

#[cfg(test)]
mod tests {
    use super::{is_grammar_attr, near_grammar_attr};

    #[test]
    fn typos_are_near_grammar_names_and_custom_marks_are_not() {
        assert_eq!(near_grammar_attr("presense"), Some("presence"));
        assert_eq!(near_grammar_attr("semanticTpye"), Some("semanticType"));
        assert_eq!(near_grammar_attr("primitivType"), Some("primitiveType"));
        assert_eq!(near_grammar_attr("byteOrdre"), Some("byteOrder"));
        for custom in [
            "scale",
            "jsonValue",
            "unit",
            "surprise",
            "mic",
            "scaleBy",
            "bogusAttr",
        ] {
            assert_eq!(near_grammar_attr(custom), None, "{custom}");
            assert!(!is_grammar_attr(custom), "{custom}");
        }
        assert!(is_grammar_attr("dimensionType"));
    }
}
