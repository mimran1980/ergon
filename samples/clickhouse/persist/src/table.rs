//! SBE schema -> ClickHouse tables, and SBE message -> RowBinary row.
//!
//! Each SBE message is one table. The column list comes from the schema IR,
//! so a field added to the XML is a column added to the table:
//!
//! | SBE                                   | ClickHouse                      |
//! |---------------------------------------|---------------------------------|
//! | integer / float / double              | `Int8`..`UInt64`, `Float32/64`  |
//! | `semanticType="UTCTimestamp"` (ns)    | `DateTime64(9, 'UTC')`          |
//! | enum                                  | `LowCardinality(String)` (name) |
//! | `char` array                          | `String` (trailing NULs cut)    |
//! | `presence="optional"`                 | `Nullable(T)`                   |
//! | group `bids { price }`                | `bids.price Array(T)`           |
//! | var-data                              | `String`                        |
//!
//! Composites, sets, non-`char` arrays and nested groups are rejected when
//! the schema is loaded, so a message that uses them is never half-recorded.

use ergo_sbe::{ByteOrder, Ir, Presence, PrimitiveType, Signal, Token};

use crate::Error;

/// Standard SBE message header: blockLength, templateId, schemaId, version.
pub(crate) const HEADER_LEN: usize = 8;

/// One recorded SBE message and the table it is written to.
#[derive(Clone, Debug)]
pub struct Table {
    /// Table name: the message name in snake_case.
    pub name: String,
    /// SBE template id.
    pub template_id: u16,
    fields: Vec<Field>,
    groups: Vec<Group>,
    var_data: Vec<VarData>,
}

/// A ClickHouse column derived from the schema.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Column {
    /// Column name (`snake_case`, groups as `group.field`).
    pub name: String,
    /// ClickHouse type, spelled the way `system.columns` reports it.
    pub ch_type: String,
}

#[derive(Clone, Debug)]
struct Field {
    column: String,
    offset: usize,
    prim: PrimitiveType,
    kind: Kind,
    /// Null sentinel for optional fields.
    null: Option<u64>,
}

#[derive(Clone, Debug)]
enum Kind {
    Number,
    Timestamp,
    Enum(Vec<(u64, String)>),
    Chars(usize),
}

#[derive(Clone, Debug)]
struct Group {
    name: String,
    count: Uint,
    block: Uint,
    header_len: usize,
    fields: Vec<Field>,
}

#[derive(Clone, Debug)]
struct VarData {
    column: String,
    length: Uint,
    header_len: usize,
}

/// An unsigned integer inside a group or var-data header: `numInGroup`,
/// `blockLength`, or a var-data `length`.
#[derive(Clone, Copy, Debug)]
struct Uint {
    offset: usize,
    prim: PrimitiveType,
}

impl Uint {
    /// Read it from the header starting at `header`.
    fn read(self, msg: &[u8], header: usize) -> Result<usize, DecodeError> {
        let at = header + self.offset;
        msg.get(at..at + self.prim.size())
            .and_then(|raw| usize::try_from(uint(raw)).ok())
            .ok_or(DecodeError("group or var-data header past end of message"))
    }
}

/// A decode failure for one message; the row is skipped, never half-written.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct DecodeError(pub &'static str);

/// Build every table in a schema.
pub fn tables_from_schema(xml: &str) -> Result<Vec<Table>, Error> {
    let mut ir = ergo_sbe::parse(xml).map_err(|e| Error::Schema(e.to_string()))?;
    ergo_sbe::resolve_schema(&mut ir, Some(xml)).map_err(|e| Error::Schema(e.to_string()))?;
    if ir.byte_order == ByteOrder::BigEndian {
        return Err(Error::Schema(
            "only littleEndian schemas are supported".into(),
        ));
    }
    message_ranges(&ir)
        .into_iter()
        .map(Table::from_tokens)
        .collect()
}

fn message_ranges(ir: &Ir) -> Vec<&[Token]> {
    let mut out = Vec::new();
    let mut start = None;
    for (i, t) in ir.tokens.iter().enumerate() {
        match t.signal {
            Signal::BeginMessage => start = Some(i),
            Signal::EndMessage => {
                if let Some(s) = start.take() {
                    out.push(&ir.tokens[s..=i]);
                }
            }
            _ => {}
        }
    }
    out
}

impl Table {
    fn from_tokens(tokens: &[Token]) -> Result<Self, Error> {
        let msg = &tokens[0];
        let mut table = Self {
            name: snake_case(&msg.name),
            template_id: msg
                .id
                .ok_or_else(|| unsupported(&msg.name, "", "a message without id"))?,
            fields: Vec::new(),
            groups: Vec::new(),
            var_data: Vec::new(),
        };
        let mut i = 1;
        while i < tokens.len() - 1 {
            let t = &tokens[i];
            let end = matching_end(tokens, i);
            match t.signal {
                Signal::BeginField => {
                    if let Some(f) = field(&tokens[i..=end], &msg.name)? {
                        table.fields.push(f);
                    }
                }
                Signal::BeginGroup => table.groups.push(group(&tokens[i..=end], &msg.name)?),
                Signal::BeginVarData => {
                    let (length, header_len) = var_header(&tokens[i..=end])
                        .ok_or_else(|| unsupported(&msg.name, &t.name, "this var-data encoding"))?;
                    table.var_data.push(VarData {
                        column: snake_case(&t.name),
                        length,
                        header_len,
                    });
                }
                _ => return Err(unsupported(&msg.name, &t.name, "this token")),
            }
            i = end + 1;
        }
        Ok(table)
    }

    /// Columns in wire order: fixed fields, group fields, var-data.
    #[must_use]
    pub(crate) fn columns(&self) -> Vec<Column> {
        let scalar = self.fields.iter().map(|f| Column {
            name: f.column.clone(),
            ch_type: f.ch_type(),
        });
        let groups = self.groups.iter().flat_map(|g| {
            g.fields.iter().map(move |f| Column {
                name: format!("{}.{}", g.name, f.column),
                ch_type: format!("Array({})", f.ch_type()),
            })
        });
        let var = self.var_data.iter().map(|v| Column {
            name: v.column.clone(),
            ch_type: "String".into(),
        });
        scalar.chain(groups).chain(var).collect()
    }

    /// Sort key: every var-data column (symbol, venue, …) then the first timestamp.
    #[must_use]
    pub(crate) fn order_by(&self) -> Vec<String> {
        let mut key: Vec<String> = self.var_data.iter().map(|v| v.column.clone()).collect();
        key.extend(self.first_timestamp());
        key
    }

    /// First required timestamp column, used for partitioning.
    #[must_use]
    pub(crate) fn first_timestamp(&self) -> Option<String> {
        self.fields
            .iter()
            .find(|f| matches!(f.kind, Kind::Timestamp) && f.null.is_none())
            .map(|f| f.column.clone())
    }

    /// Append `msg` (header included) as one RowBinary row, writing only the
    /// columns whose `include` flag is set. On error `out` is left unchanged.
    pub(crate) fn write_row(
        &self,
        msg: &[u8],
        include: &[bool],
        out: &mut Vec<u8>,
    ) -> Result<(), DecodeError> {
        let mark = out.len();
        let result = self.write_row_inner(msg, include, out);
        if result.is_err() {
            out.truncate(mark);
        }
        result
    }

    fn write_row_inner(
        &self,
        msg: &[u8],
        include: &[bool],
        out: &mut Vec<u8>,
    ) -> Result<(), DecodeError> {
        let acting_block = usize::from(u16::from_le_bytes(bytes::<2>(msg, 0)?));
        let body = HEADER_LEN;
        let mut col = 0;
        for f in &self.fields {
            if include[col] {
                f.write(msg, body, body + acting_block, out)?;
            }
            col += 1;
        }
        let mut pos = body + acting_block;
        for g in &self.groups {
            let count = g.count.read(msg, pos)?;
            let block = g.block.read(msg, pos)?;
            let first = pos + g.header_len;
            let end = count
                .checked_mul(block)
                .and_then(|n| n.checked_add(first))
                .ok_or(DecodeError("group size"))?;
            if end > msg.len() {
                return Err(DecodeError("group past end of message"));
            }
            for f in &g.fields {
                if include[col] {
                    write_varint(count as u64, out);
                    for e in 0..count {
                        let entry = first + e * block;
                        f.write(msg, entry, entry + block, out)?;
                    }
                }
                col += 1;
            }
            pos = end;
        }
        for v in &self.var_data {
            let len = v.length.read(msg, pos)?;
            let start = pos + v.header_len;
            let data = msg
                .get(start..start + len)
                .ok_or(DecodeError("var-data past end of message"))?;
            if include[col] {
                write_string(data, out);
            }
            col += 1;
            pos = start + len;
        }
        Ok(())
    }
}

impl Field {
    fn ch_type(&self) -> String {
        let base = match &self.kind {
            Kind::Number => number_type(self.prim).to_string(),
            Kind::Timestamp => "DateTime64(9, 'UTC')".to_string(),
            Kind::Enum(_) => return "LowCardinality(String)".to_string(),
            Kind::Chars(_) => "String".to_string(),
        };
        if self.null.is_some() {
            format!("Nullable({base})")
        } else {
            base
        }
    }

    /// Write this field from the block `[start, end)`.
    fn write(
        &self,
        msg: &[u8],
        start: usize,
        end: usize,
        out: &mut Vec<u8>,
    ) -> Result<(), DecodeError> {
        let at = start + self.offset;
        let width = match self.kind {
            Kind::Chars(n) => n,
            _ => self.prim.size(),
        };
        // The writer encodes with the same schema, so every field is inside
        // its block; anything else is a message from another schema version.
        if at + width > end {
            return Err(DecodeError("field outside its block"));
        }
        let raw = msg
            .get(at..at + width)
            .ok_or(DecodeError("field past end of message"))?;
        match &self.kind {
            Kind::Enum(values) => match values.iter().find(|(k, _)| *k == uint(raw)) {
                Some((_, name)) => write_string(name.as_bytes(), out),
                None => write_string(uint(raw).to_string().as_bytes(), out),
            },
            Kind::Chars(_) => {
                let cut = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
                write_string(&raw[..cut], out);
            }
            Kind::Number | Kind::Timestamp => {
                if let Some(null) = self.null {
                    let is_null = self.is_null(raw, null);
                    out.push(u8::from(is_null));
                    if is_null {
                        return Ok(());
                    }
                }
                out.extend_from_slice(raw);
            }
        }
        Ok(())
    }

    fn is_null(&self, raw: &[u8], null: u64) -> bool {
        match self.prim {
            PrimitiveType::Float => f32::from_bits(uint(raw) as u32).is_nan(),
            PrimitiveType::Double => f64::from_bits(uint(raw)).is_nan(),
            _ => {
                let bits = raw.len() * 8;
                let mask = if bits == 64 {
                    u64::MAX
                } else {
                    (1 << bits) - 1
                };
                uint(raw) == null & mask
            }
        }
    }
}

/// Parse one `BeginField..EndField` span. `None` for constant fields (not on the wire).
fn field(tokens: &[Token], message: &str) -> Result<Option<Field>, Error> {
    let t = &tokens[0];
    let fail = |what: &str| unsupported(message, &t.name, what);
    if t.encoding.presence == Presence::Constant {
        return Ok(None);
    }
    let offset = t
        .encoding
        .offset
        .ok_or_else(|| fail("a field without offset"))?;
    let null =
        (t.encoding.presence == Presence::Optional).then_some(t.encoding.null_value.unwrap_or(0));
    let inner = tokens.get(1).ok_or_else(|| fail("an empty field"))?;
    let (prim, kind) = match (t.encoding.primitive_type, inner.signal) {
        (Some(prim), _) => match (prim, t.encoding.length) {
            (PrimitiveType::Char, Some(n)) if n > 1 => (prim, Kind::Chars(n)),
            (_, Some(n)) if n > 1 => return Err(fail("a non-char array")),
            _ if t.encoding.semantic_type.as_deref() == Some("UTCTimestamp") => match prim {
                PrimitiveType::Int64 | PrimitiveType::UInt64 => (prim, Kind::Timestamp),
                _ => return Err(fail("a UTCTimestamp that is not 64-bit")),
            },
            _ => (prim, Kind::Number),
        },
        (None, Signal::BeginEnum) => {
            let prim = inner
                .encoding
                .primitive_type
                .ok_or_else(|| fail("an enum without encoding"))?;
            let values = tokens
                .iter()
                .filter(|v| {
                    v.signal == Signal::Encoding && v.encoding.presence == Presence::Constant
                })
                .filter_map(|v| {
                    Some((
                        enum_value(v.encoding.constant_value.as_deref()?, prim)?,
                        v.name.clone(),
                    ))
                })
                .collect();
            (prim, Kind::Enum(values))
        }
        (None, Signal::BeginComposite) => return Err(fail("a composite")),
        (None, Signal::BeginSet) => return Err(fail("a set")),
        _ => return Err(fail("this field type")),
    };
    Ok(Some(Field {
        column: snake_case(&t.name),
        offset,
        prim,
        kind,
        null,
    }))
}

fn group(tokens: &[Token], message: &str) -> Result<Group, Error> {
    let name = snake_case(&tokens[0].name);
    let fail = |what: &str| unsupported(message, &name, what);
    let mut dimension = Vec::new();
    let mut fields = Vec::new();
    let mut i = 1;
    while i < tokens.len() - 1 {
        let end = matching_end(tokens, i);
        let t = &tokens[i];
        match t.signal {
            Signal::BeginComposite if fields.is_empty() && dimension.is_empty() => {
                dimension = tokens[i..=end]
                    .iter()
                    .filter(|d| d.signal == Signal::BeginField)
                    .collect();
            }
            Signal::BeginField => {
                if let Some(f) = field(&tokens[i..=end], message)? {
                    fields.push(f);
                }
            }
            Signal::BeginGroup => return Err(fail("a nested group")),
            Signal::BeginVarData => return Err(fail("var-data inside a group")),
            _ => {}
        }
        i = end + 1;
    }
    let count = member(dimension.iter().copied(), "numInGroup")
        .ok_or_else(|| fail("a group without numInGroup"))?;
    let block = member(dimension.iter().copied(), "blockLength")
        .ok_or_else(|| fail("a group without blockLength"))?;
    let header_len = dimension
        .iter()
        .filter_map(|d| Some(d.encoding.offset? + d.encoding.primitive_type?.size()))
        .max()
        .ok_or_else(|| fail("an empty group dimension"))?;
    Ok(Group {
        name,
        count,
        block,
        header_len,
        fields,
    })
}

/// The var-data `length` member and the offset of its bytes.
fn var_header(tokens: &[Token]) -> Option<(Uint, usize)> {
    Some((member(tokens, "length")?, member(tokens, "varData")?.offset))
}

/// The `name` field (offset and type) among `tokens`.
fn member<'a>(tokens: impl IntoIterator<Item = &'a Token>, name: &str) -> Option<Uint> {
    let t = tokens
        .into_iter()
        .find(|t| t.signal == Signal::BeginField && t.name == name)?;
    Some(Uint {
        offset: t.encoding.offset?,
        prim: t.encoding.primitive_type?,
    })
}

/// `Message.name: what is not supported`.
fn unsupported(message: &str, name: &str, what: &str) -> Error {
    Error::Schema(format!("{message}.{name}: {what} is not supported"))
}

/// Index of the token closing the one at `i` (itself for leaf tokens).
fn matching_end(tokens: &[Token], i: usize) -> usize {
    let close = match tokens[i].signal {
        Signal::BeginField => Signal::EndField,
        Signal::BeginGroup => Signal::EndGroup,
        Signal::BeginVarData => Signal::EndVarData,
        Signal::BeginComposite => Signal::EndComposite,
        Signal::BeginEnum => Signal::EndEnum,
        Signal::BeginSet => Signal::EndSet,
        _ => return i,
    };
    let open = tokens[i].signal;
    let mut depth = 0;
    for (j, t) in tokens.iter().enumerate().skip(i) {
        if t.signal == open {
            depth += 1;
        } else if t.signal == close {
            depth -= 1;
            if depth == 0 {
                return j;
            }
        }
    }
    tokens.len() - 1
}

fn number_type(prim: PrimitiveType) -> &'static str {
    match prim {
        PrimitiveType::Int8 => "Int8",
        PrimitiveType::Int16 => "Int16",
        PrimitiveType::Int32 => "Int32",
        PrimitiveType::Int64 => "Int64",
        PrimitiveType::Char | PrimitiveType::UInt8 => "UInt8",
        PrimitiveType::UInt16 => "UInt16",
        PrimitiveType::UInt32 => "UInt32",
        PrimitiveType::UInt64 => "UInt64",
        PrimitiveType::Float => "Float32",
        PrimitiveType::Double => "Float64",
    }
}

fn enum_value(text: &str, prim: PrimitiveType) -> Option<u64> {
    if prim == PrimitiveType::Char && text.len() == 1 {
        return Some(u64::from(text.as_bytes()[0]));
    }
    text.parse::<u64>()
        .ok()
        .or_else(|| text.parse::<i64>().ok().map(|v| v as u64))
}

fn bytes<const N: usize>(msg: &[u8], at: usize) -> Result<[u8; N], DecodeError> {
    msg.get(at..at + N)
        .and_then(|s| s.try_into().ok())
        .ok_or(DecodeError("message shorter than its header"))
}

/// Little-endian unsigned value of up to 8 bytes.
fn uint(raw: &[u8]) -> u64 {
    let mut le = [0u8; 8];
    le[..raw.len()].copy_from_slice(raw);
    u64::from_le_bytes(le)
}

fn write_varint(mut v: u64, out: &mut Vec<u8>) {
    while v >= 0x80 {
        out.push((v as u8) | 0x80);
        v >>= 7;
    }
    out.push(v as u8);
}

fn write_string(data: &[u8], out: &mut Vec<u8>) {
    write_varint(data.len() as u64, out);
    out.extend_from_slice(data);
}

/// `bidPrice` -> `bid_price`, `BookSnapshot` -> `book_snapshot`.
#[must_use]
pub(crate) fn snake_case(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    for (i, c) in name.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0 && !out.ends_with('_') && !out.ends_with('.') {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn snake_case_names() -> TestResult {
        assert_eq!(snake_case("BookSnapshot"), "book_snapshot");
        assert_eq!(snake_case("bidPrice"), "bid_price");
        assert_eq!(snake_case("tsEvent"), "ts_event");
        assert_eq!(snake_case("price"), "price");
        Ok(())
    }

    #[test]
    fn varint_matches_leb128() -> TestResult {
        let mut out = Vec::new();
        write_varint(300, &mut out);
        assert_eq!(out, [0xAC, 0x02]);
        Ok(())
    }
}
