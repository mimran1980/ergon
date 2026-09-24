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
    big_endian: bool,
}

/// A ClickHouse column derived from the schema.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Column {
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
    /// Offset/type of `numInGroup` and `blockLength` inside the dimension.
    count: (usize, PrimitiveType),
    block: (usize, PrimitiveType),
    header_len: usize,
    fields: Vec<Field>,
}

#[derive(Clone, Debug)]
struct VarData {
    column: String,
    length: (usize, PrimitiveType),
    header_len: usize,
}

/// A decode failure for one message; the row is skipped, never half-written.
#[derive(Debug, PartialEq, Eq)]
pub struct DecodeError(pub &'static str);

/// Build every table in a schema.
pub fn tables_from_schema(xml: &str) -> Result<Vec<Table>, Error> {
    let mut ir = ergo_sbe::parse(xml).map_err(|e| Error::Schema(e.to_string()))?;
    ergo_sbe::resolve_schema(&mut ir, Some(xml)).map_err(|e| Error::Schema(e.to_string()))?;
    let big_endian = ir.byte_order == ByteOrder::BigEndian;
    message_ranges(&ir)
        .into_iter()
        .map(|tokens| Table::from_tokens(tokens, big_endian))
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
    fn from_tokens(tokens: &[Token], big_endian: bool) -> Result<Self, Error> {
        let msg = &tokens[0];
        let unsupported = |what: &str, name: &str| {
            Error::Schema(format!("{}.{name}: {what} is not supported", msg.name))
        };
        let mut table = Self {
            name: snake_case(&msg.name),
            template_id: msg
                .id
                .ok_or_else(|| unsupported("a message without id", ""))?,
            fields: Vec::new(),
            groups: Vec::new(),
            var_data: Vec::new(),
            big_endian,
        };
        let mut i = 1;
        while i < tokens.len() - 1 {
            let t = &tokens[i];
            let end = matching_end(tokens, i);
            match t.signal {
                Signal::BeginField => {
                    if let Some(f) = field(&tokens[i..=end], "", &msg.name)? {
                        table.fields.push(f);
                    }
                }
                Signal::BeginGroup => table.groups.push(group(&tokens[i..=end], &msg.name)?),
                Signal::BeginVarData => {
                    let (length, header_len) = var_header(&tokens[i..=end])
                        .ok_or_else(|| unsupported("this var-data encoding", &t.name))?;
                    table.var_data.push(VarData {
                        column: snake_case(&t.name),
                        length,
                        header_len,
                    });
                }
                _ => return Err(unsupported("this token", &t.name)),
            }
            i = end + 1;
        }
        Ok(table)
    }

    /// Columns in wire order: fixed fields, group fields, var-data.
    #[must_use]
    pub fn columns(&self) -> Vec<Column> {
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
    pub fn order_by(&self) -> Vec<String> {
        let mut key: Vec<String> = self.var_data.iter().map(|v| v.column.clone()).collect();
        key.extend(self.first_timestamp());
        key
    }

    /// First required timestamp column, used for partitioning.
    #[must_use]
    pub fn first_timestamp(&self) -> Option<String> {
        self.fields
            .iter()
            .find(|f| matches!(f.kind, Kind::Timestamp) && f.null.is_none())
            .map(|f| f.column.clone())
    }

    /// Append `msg` (header included) as one RowBinary row, writing only the
    /// columns whose `include` flag is set. On error `out` is left unchanged.
    pub fn write_row(
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
                f.write(msg, body, body + acting_block, self.big_endian, out)?;
            }
            col += 1;
        }
        let mut pos = body + acting_block;
        for g in &self.groups {
            let count = read_uint(msg, pos + g.count.0, g.count.1, self.big_endian)?;
            let block = read_uint(msg, pos + g.block.0, g.block.1, self.big_endian)?;
            let count = usize::try_from(count).map_err(|_| DecodeError("group count"))?;
            let block = usize::try_from(block).map_err(|_| DecodeError("group block length"))?;
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
                        f.write(msg, entry, entry + block, self.big_endian, out)?;
                    }
                }
                col += 1;
            }
            pos = end;
        }
        for v in &self.var_data {
            let len = read_uint(msg, pos + v.length.0, v.length.1, self.big_endian)?;
            let len = usize::try_from(len).map_err(|_| DecodeError("var-data length"))?;
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

    /// Write this field from the block `[start, end)`. A field past the
    /// acting block (an older writer) is written as NULL / default.
    fn write(
        &self,
        msg: &[u8],
        start: usize,
        end: usize,
        big: bool,
        out: &mut Vec<u8>,
    ) -> Result<(), DecodeError> {
        let at = start + self.offset;
        let width = match self.kind {
            Kind::Chars(n) => n,
            _ => self.prim.size(),
        };
        let raw = if at + width <= end {
            Some(
                msg.get(at..at + width)
                    .ok_or(DecodeError("field past end of message"))?,
            )
        } else {
            None
        };
        if let Kind::Enum(values) = &self.kind {
            let name = match raw {
                Some(r) => {
                    let v = uint(r, big);
                    values
                        .iter()
                        .find(|(k, _)| *k == v)
                        .map_or_else(|| v.to_string(), |(_, n)| n.clone())
                }
                None => String::new(),
            };
            write_string(name.as_bytes(), out);
            return Ok(());
        }
        if let Kind::Chars(_) = self.kind {
            let text = raw.unwrap_or_default();
            let cut = text.iter().position(|&b| b == 0).unwrap_or(text.len());
            write_string(&text[..cut], out);
            return Ok(());
        }
        if let Some(null) = self.null {
            let is_null = raw.is_none_or(|r| self.is_null(r, null, big));
            out.push(u8::from(is_null));
            if is_null {
                return Ok(());
            }
        }
        match raw {
            Some(r) if big => out.extend(r.iter().rev()),
            Some(r) => out.extend_from_slice(r),
            None => out.extend(std::iter::repeat_n(0, width)),
        }
        Ok(())
    }

    fn is_null(&self, raw: &[u8], null: u64, big: bool) -> bool {
        match self.prim {
            PrimitiveType::Float => f32::from_bits(uint(raw, big) as u32).is_nan(),
            PrimitiveType::Double => f64::from_bits(uint(raw, big)).is_nan(),
            _ => {
                let bits = raw.len() * 8;
                let mask = if bits == 64 {
                    u64::MAX
                } else {
                    (1 << bits) - 1
                };
                uint(raw, big) == null & mask
            }
        }
    }
}

/// Parse one `BeginField..EndField` span. `None` for constant fields (not on the wire).
fn field(tokens: &[Token], prefix: &str, message: &str) -> Result<Option<Field>, Error> {
    let t = &tokens[0];
    let name = format!("{prefix}{}", t.name);
    let fail = |what: &str| Error::Schema(format!("{message}.{name}: {what} is not supported"));
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
        column: snake_case(&name),
        offset,
        prim,
        kind,
        null,
    }))
}

fn group(tokens: &[Token], message: &str) -> Result<Group, Error> {
    let name = snake_case(&tokens[0].name);
    let fail = |what: &str| Error::Schema(format!("{message}.{name}: {what} is not supported"));
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
            // Some IRs list the dimension members directly.
            Signal::BeginField if t.id.is_none() && fields.is_empty() => dimension.push(t),
            Signal::BeginField => {
                if let Some(f) = field(&tokens[i..=end], "", message)? {
                    fields.push(f);
                }
            }
            Signal::BeginGroup => return Err(fail("a nested group")),
            Signal::BeginVarData => return Err(fail("var-data inside a group")),
            _ => {}
        }
        i = end + 1;
    }
    let member = |n: &str| {
        dimension
            .iter()
            .find(|d| d.name == n)
            .and_then(|d| Some((d.encoding.offset?, d.encoding.primitive_type?)))
    };
    let count = member("numInGroup").ok_or_else(|| fail("a group without numInGroup"))?;
    let block = member("blockLength").ok_or_else(|| fail("a group without blockLength"))?;
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

fn var_header(tokens: &[Token]) -> Option<((usize, PrimitiveType), usize)> {
    let member = |n: &str| {
        tokens
            .iter()
            .find(|t| t.signal == Signal::BeginField && t.name == n)
    };
    let length = member("length")?;
    let data = member("varData")?;
    Some((
        (length.encoding.offset?, length.encoding.primitive_type?),
        data.encoding.offset?,
    ))
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

fn read_uint(msg: &[u8], at: usize, prim: PrimitiveType, big: bool) -> Result<u64, DecodeError> {
    msg.get(at..at + prim.size())
        .map(|r| uint(r, big))
        .ok_or(DecodeError("dimension past end of message"))
}

fn uint(raw: &[u8], big: bool) -> u64 {
    let mut v = 0u64;
    for (i, &b) in raw.iter().enumerate() {
        let shift = if big { (raw.len() - 1 - i) * 8 } else { i * 8 };
        v |= u64::from(b) << shift;
    }
    v
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
pub fn snake_case(name: &str) -> String {
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

    #[test]
    fn snake_case_names() {
        assert_eq!(snake_case("BookSnapshot"), "book_snapshot");
        assert_eq!(snake_case("bidPrice"), "bid_price");
        assert_eq!(snake_case("tsEvent"), "ts_event");
        assert_eq!(snake_case("price"), "price");
    }

    #[test]
    fn varint_matches_leb128() {
        let mut out = Vec::new();
        write_varint(300, &mut out);
        assert_eq!(out, [0xAC, 0x02]);
    }
}
