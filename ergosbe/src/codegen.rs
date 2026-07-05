//! Rust code generation boundary.

use crate::{GenerationConfig, Schema};
use crate::ir::{ByteOrder, Presence, PrimitiveType, Signal, Token};

/// A single generated Rust module.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedModule {
    /// Relative module path, for example `messages.rs`.
    pub path: String,
    /// Rust source code.
    pub source: String,
}

/// Complete generated output for a schema.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GeneratedModuleSet {
    modules: Vec<GeneratedModule>,
}

impl GeneratedModuleSet {
    /// Add a generated module to the set.
    pub fn push(&mut self, module: GeneratedModule) {
        self.modules.push(module);
    }

    /// Iterate over generated modules in deterministic output order.
    #[must_use]
    pub fn modules(&self) -> impl ExactSizeIterator<Item = &GeneratedModule> {
        self.modules.iter()
    }
}

/// SBE-to-Rust generator.
#[derive(Clone, Debug)]
pub struct Generator {
    config: GenerationConfig,
}

impl Generator {
    /// Create a generator with the supplied configuration.
    #[must_use]
    pub const fn new(config: GenerationConfig) -> Self {
        Self { config }
    }

    /// Return this generator's configuration.
    #[must_use]
    pub const fn config(&self) -> &GenerationConfig {
        &self.config
    }

    /// Generate Rust modules for a normalized schema.
    #[must_use]
    pub fn generate(&self, schema: &Schema) -> GeneratedModuleSet {
        let mut modules = GeneratedModuleSet::default();
        let ir = &schema.ir;

        let mut src = String::new();
        src.push_str(&format!(
            "//! Generated from SBE schema package `{}` id {} version {}.\n\n",
            schema.package, schema.id, schema.version
        ));

        src.push_str("#![allow(non_camel_case_types)]\n");
        src.push_str("#![allow(non_snake_case)]\n");
        src.push_str("#![allow(clippy::identity_op)]\n");
        src.push_str("#![allow(clippy::eq_op)]\n");
        src.push_str("#![allow(clippy::needless_borrow)]\n");
        src.push_str("#![allow(clippy::manual_range_contains)]\n");
        src.push_str("#![allow(unused_imports)]\n");
        src.push_str("#![allow(unused_variables)]\n");
        src.push_str("#![allow(unused_mut)]\n");
        src.push_str("#![allow(dead_code)]\n\n");

        // 1. Generate inline SBE runtime
        src.push_str("pub mod sbe_rt {\n");
        src.push_str("    #[derive(Debug, Clone, Copy, PartialEq, Eq)]\n");
        src.push_str("    pub enum DecodeError {\n");
        src.push_str("        BufferTooShort { needed: usize, available: usize },\n");
        src.push_str("        WrongSchema { expected: u16, actual: u16 },\n");
        src.push_str("        UnknownTemplateLength { template_id: u16 },\n");
        src.push_str("        InvalidVarDataLength { field: &'static str, length: u32 },\n");
        src.push_str("        Utf8(core::str::Utf8Error),\n");
        src.push_str("    }\n\n");
        src.push_str("    impl core::fmt::Display for DecodeError {\n");
        src.push_str("        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {\n");
        src.push_str("            match self {\n");
        src.push_str("                Self::BufferTooShort { needed, available } => write!(f, \"buffer too short: needed {}, available {}\", needed, available),\n");
        src.push_str("                Self::WrongSchema { expected, actual } => write!(f, \"wrong schema id: expected {}, actual {}\", expected, actual),\n");
        src.push_str("                Self::UnknownTemplateLength { template_id } => write!(f, \"unknown template length for template id {}\", template_id),\n");
        src.push_str("                Self::InvalidVarDataLength { field, length } => write!(f, \"invalid var data length for field {}: {}\", field, length),\n");
        src.push_str("                Self::Utf8(err) => write!(f, \"UTF-8 decode error: {}\", err),\n");
        src.push_str("            }\n");
        src.push_str("        }\n");
        src.push_str("    }\n\n");
        src.push_str("    impl core::error::Error for DecodeError {}\n\n");
        src.push_str("    #[derive(Debug, Clone, Copy, PartialEq, Eq)]\n");
        src.push_str("    pub enum EncodeError {\n");
        src.push_str("        BufferTooShort { needed: usize, available: usize },\n");
        src.push_str("    }\n\n");
        src.push_str("    impl core::fmt::Display for EncodeError {\n");
        src.push_str("        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {\n");
        src.push_str("            match self {\n");
        src.push_str("                Self::BufferTooShort { needed, available } => write!(f, \"buffer too short: needed {}, available {}\", needed, available),\n");
        src.push_str("            }\n");
        src.push_str("        }\n");
        src.push_str("    }\n\n");
        src.push_str("    impl core::error::Error for EncodeError {}\n\n");
        src.push_str("    pub trait SbeMessage {\n");
        src.push_str("        const TEMPLATE_ID: u16;\n");
        src.push_str("        const BLOCK_LENGTH: usize;\n");
        src.push_str("        const SCHEMA_ID: u16;\n");
        src.push_str("        const SCHEMA_VERSION: u16;\n");
        src.push_str("    }\n\n");
        src.push_str("    pub mod private {\n");
        src.push_str("        pub trait Sealed {}\n");
        src.push_str("    }\n\n");
        src.push_str("    pub trait EncodeGroupEntry<E> {\n");
        src.push_str("        fn encode(self, entry: &mut E);\n");
        src.push_str("    }\n\n");
        src.push_str("    impl<E, F> EncodeGroupEntry<E> for F\n");
        src.push_str("    where\n");
        src.push_str("        F: FnOnce(&mut E),\n");
        src.push_str("    {\n");
        src.push_str("        #[inline]\n");
        src.push_str("        fn encode(self, entry: &mut E) {\n");
        src.push_str("            self(entry);\n");
        src.push_str("        }\n");
        src.push_str("    }\n");
        src.push_str("}\n\n");

        // 2. Group the tokens into composites, enums, sets, and messages
        let elements = partition_tokens(&ir.tokens);

        // 3. Generate Enums
        for enum_tokens in &elements.enums {
            generate_enum(&mut src, enum_tokens);
        }

        // 4. Generate Sets/Choices
        for set_tokens in &elements.sets {
            generate_set(&mut src, set_tokens);
        }

        // 5. Generate Composites
        for composite_tokens in &elements.composites {
            generate_composite(&mut src, composite_tokens, ir.byte_order);
        }

        // Generate MessageHeader alias if custom name is used
        let header_pascal = to_pascal_case(&ir.header_type);
        if header_pascal != "MessageHeader" {
            src.push_str(&format!("pub type MessageHeader = {};\n\n", header_pascal));
        }

        // 6. Generate Messages (Decoders and Encoders)
        let messages: Vec<MessageStructure> = elements
            .messages
            .iter()
            .map(|toks| parse_message_structure(toks, &elements))
            .collect();

        for msg in &messages {
            generate_message_decoder(&mut src, msg, &elements, ir.byte_order, ir.id, ir.version, &ir.header_type);
            generate_message_encoder(&mut src, msg, &elements, ir.byte_order, ir.id, ir.version, &ir.header_type);
        }

        // 7. Generate AnyMessage enum
        generate_any_message(&mut src, &messages, &elements, ir.id, &ir.header_type);

        // Format through syn/prettyplease
        let source = syn::parse_str::<syn::File>(&src)
            .map(|file| prettyplease::unparse(&file))
            .unwrap_or(src);

        modules.push(GeneratedModule {
            path: format!("{}.rs", self.config.module_name),
            source,
        });

        modules
    }
}

fn to_pascal_case(s: &str) -> String {
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
    res
}

fn to_snake_case(s: &str) -> String {
    let mut res = String::new();
    let mut prev_is_lower = false;
    let mut prev_is_upper = false;
    for c in s.chars() {
        if c == '_' || c == '-' || c == ' ' {
            res.push('_');
            prev_is_lower = false;
            prev_is_upper = false;
        } else if c.is_uppercase() {
            if prev_is_lower {
                res.push('_');
            }
            res.extend(c.to_lowercase());
            prev_is_lower = false;
            prev_is_upper = true;
        } else {
            res.push(c);
            prev_is_lower = true;
            prev_is_upper = false;
        }
    }
    let mut clean = String::new();
    for c in res.chars() {
        if c == '_' && clean.ends_with('_') {
            continue;
        }
        clean.push(c);
    }
    clean
}

fn to_upper_snake_case(s: &str) -> String {
    to_snake_case(s).to_uppercase()
}

fn format_discriminant(val: &str, is_char: bool) -> String {
    if is_char {
        if val.len() == 1 {
            format!("b'{}'", val)
        } else if let Ok(n) = val.parse::<u8>() {
            format!("{}", n)
        } else {
            format!("{}", val.as_bytes().first().copied().unwrap_or(0))
        }
    } else {
        if let Ok(n) = val.parse::<i128>() {
            format!("{}", n)
        } else if val.len() == 1 {
            format!("b'{}'", val)
        } else {
            val.to_string()
        }
    }
}

fn constant_value_expr(prim: PrimitiveType, val: &str) -> String {
    match prim {
        PrimitiveType::Char => {
            if val.len() == 1 {
                format!("b'{}'", val)
            } else {
                format!("{:?}", val)
            }
        }
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

fn find_matching_end(tokens: &[Token], start: usize, begin: Signal, end: Signal) -> usize {
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

struct SchemaElements {
    composites: Vec<Vec<Token>>,
    enums: Vec<Vec<Token>>,
    sets: Vec<Vec<Token>>,
    messages: Vec<Vec<Token>>,
}

fn partition_tokens(tokens: &[Token]) -> SchemaElements {
    let mut composites = Vec::new();
    let mut enums = Vec::new();
    let mut sets = Vec::new();
    let mut messages = Vec::new();

    let mut i = 0;
    while i < tokens.len() {
        match tokens[i].signal {
            Signal::BeginComposite => {
                let end = find_matching_end(tokens, i, Signal::BeginComposite, Signal::EndComposite);
                composites.push(tokens[i..=end].to_vec());
                i = end + 1;
            }
            Signal::BeginEnum => {
                let end = find_matching_end(tokens, i, Signal::BeginEnum, Signal::EndEnum);
                enums.push(tokens[i..=end].to_vec());
                i = end + 1;
            }
            Signal::BeginSet => {
                let end = find_matching_end(tokens, i, Signal::BeginSet, Signal::EndSet);
                sets.push(tokens[i..=end].to_vec());
                i = end + 1;
            }
            Signal::BeginMessage => {
                let end = find_matching_end(tokens, i, Signal::BeginMessage, Signal::EndMessage);
                messages.push(tokens[i..=end].to_vec());
                i = end + 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    SchemaElements {
        composites,
        enums,
        sets,
        messages,
    }
}

struct MessageStructure {
    name: String,
    id: u16,
    since_version: u16,
    description: Option<String>,
    semantic_type: Option<String>,
    fields: Vec<MessageField>,
    groups: Vec<MessageGroup>,
    var_data: Vec<MessageVarData>,
}

#[derive(Clone)]
struct MessageField {
    name: String,
    id: Option<u16>,
    offset: usize,
    presence: Presence,
    since_version: u16,
    null_value: Option<u64>,
    min_value: Option<u64>,
    max_value: Option<u64>,
    description: Option<String>,
    semantic_type: Option<String>,
    constant_value: Option<String>,
    field_type: FieldType,
}

#[derive(Clone)]
enum FieldType {
    Primitive(PrimitiveType, Option<usize>),
    Composite { name: String, size: usize },
    Enum { name: String, encoding_type: PrimitiveType },
    Set { name: String, encoding_type: PrimitiveType },
}

#[derive(Clone)]
struct MessageGroup {
    name: String,
    id: u16,
    since_version: u16,
    description: Option<String>,
    dimension_type: String,
    fields: Vec<MessageField>,
    groups: Vec<MessageGroup>,
    var_data: Vec<MessageVarData>,
    block_length: usize,
}

#[derive(Clone)]
struct MessageVarData {
    name: String,
    id: u16,
    since_version: u16,
    description: Option<String>,
    type_name: String,
}

fn parse_message_structure(tokens: &[Token], elements: &SchemaElements) -> MessageStructure {
    let begin_token = &tokens[0];
    let name = begin_token.name.clone();
    let id = begin_token.id.unwrap_or(0);
    let since_version = begin_token.encoding.since_version;
    let description = begin_token.encoding.description.clone();
    let semantic_type = begin_token.encoding.semantic_type.clone();

    let mut fields = Vec::new();
    let mut groups = Vec::new();
    let mut var_data = Vec::new();

    let mut i = 1;
    let end_limit = tokens.len() - 1;
    while i < end_limit {
        match tokens[i].signal {
            Signal::BeginField => {
                let end = find_matching_end(tokens, i, Signal::BeginField, Signal::EndField);
                let f = parse_field_structure(&tokens[i..=end], elements);
                fields.push(f);
                i = end + 1;
            }
            Signal::BeginGroup => {
                let end = find_matching_end(tokens, i, Signal::BeginGroup, Signal::EndGroup);
                let g = parse_group_structure(&tokens[i..=end], elements);
                groups.push(g);
                i = end + 1;
            }
            Signal::BeginVarData => {
                let end = find_matching_end(tokens, i, Signal::BeginVarData, Signal::EndVarData);
                let vd = parse_vardata_structure(&tokens[i..=end]);
                var_data.push(vd);
                i = end + 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    MessageStructure {
        name,
        id,
        since_version,
        description,
        semantic_type,
        fields,
        groups,
        var_data,
    }
}

fn parse_field_structure(tokens: &[Token], elements: &SchemaElements) -> MessageField {
    let begin = &tokens[0];
    let name = begin.name.clone();
    let id = begin.id;
    let offset = begin.encoding.offset.unwrap_or(0);
    let presence = begin.encoding.presence;
    let since_version = begin.encoding.since_version;
    let null_value = begin.encoding.null_value;
    let min_value = begin.encoding.min_value;
    let max_value = begin.encoding.max_value;
    let description = begin.encoding.description.clone();
    let semantic_type = begin.encoding.semantic_type.clone();
    let constant_value = begin.encoding.constant_value.clone();

    let field_type = if tokens.len() > 2 {
        let inner_signal = tokens[1].signal;
        let inner_name = tokens[1].name.clone();
        match inner_signal {
            Signal::BeginComposite => {
                let size = elements.composites.iter()
                    .find(|c| c[0].name == inner_name)
                    .and_then(|c| c[0].encoding.offset)
                    .unwrap_or(0);
                FieldType::Composite { name: inner_name, size }
            }
            Signal::BeginEnum => {
                let encoding_type = tokens[1].encoding.primitive_type.unwrap_or(PrimitiveType::UInt8);
                FieldType::Enum { name: inner_name, encoding_type }
            }
            Signal::BeginSet => {
                let encoding_type = tokens[1].encoding.primitive_type.unwrap_or(PrimitiveType::UInt8);
                FieldType::Set { name: inner_name, encoding_type }
            }
            _ => {
                FieldType::Primitive(begin.encoding.primitive_type.unwrap_or(PrimitiveType::UInt8), begin.encoding.length)
            }
        }
    } else {
        FieldType::Primitive(begin.encoding.primitive_type.unwrap_or(PrimitiveType::UInt8), begin.encoding.length)
    };

    MessageField {
        name,
        id,
        offset,
        presence,
        since_version,
        null_value,
        min_value,
        max_value,
        description,
        semantic_type,
        constant_value,
        field_type,
    }
}

fn parse_group_structure(tokens: &[Token], elements: &SchemaElements) -> MessageGroup {
    let begin = &tokens[0];
    let name = begin.name.clone();
    let id = begin.id.unwrap_or(0);
    let since_version = begin.encoding.since_version;
    let description = begin.encoding.description.clone();
    let block_length = begin.encoding.offset.unwrap_or(0);

    let mut dimension_type = "groupSizeEncoding".to_string();
    let mut fields = Vec::new();
    let mut groups = Vec::new();
    let mut var_data = Vec::new();

    let mut i = 1;
    if tokens[i].signal == Signal::BeginComposite {
        dimension_type = tokens[i].name.clone();
        let dim_end = find_matching_end(tokens, i, Signal::BeginComposite, Signal::EndComposite);
        i = dim_end + 1;
    }

    let end_limit = tokens.len() - 1;
    while i < end_limit {
        match tokens[i].signal {
            Signal::BeginField => {
                let end = find_matching_end(tokens, i, Signal::BeginField, Signal::EndField);
                fields.push(parse_field_structure(&tokens[i..=end], elements));
                i = end + 1;
            }
            Signal::BeginGroup => {
                let end = find_matching_end(tokens, i, Signal::BeginGroup, Signal::EndGroup);
                groups.push(parse_group_structure(&tokens[i..=end], elements));
                i = end + 1;
            }
            Signal::BeginVarData => {
                let end = find_matching_end(tokens, i, Signal::BeginVarData, Signal::EndVarData);
                var_data.push(parse_vardata_structure(&tokens[i..=end]));
                i = end + 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    MessageGroup {
        name,
        id,
        since_version,
        description,
        dimension_type,
        fields,
        groups,
        var_data,
        block_length,
    }
}

fn parse_vardata_structure(tokens: &[Token]) -> MessageVarData {
    let begin = &tokens[0];
    let name = begin.name.clone();
    let id = begin.id.unwrap_or(0);
    let since_version = begin.encoding.since_version;
    let description = begin.encoding.description.clone();

    let mut type_name = "varDataEncoding".to_string();
    if tokens.len() > 2 && tokens[1].signal == Signal::BeginComposite {
        type_name = tokens[1].name.clone();
    }

    MessageVarData {
        name,
        id,
        since_version,
        description,
        type_name,
    }
}

fn rust_type(prim: PrimitiveType) -> &'static str {
    match prim {
        PrimitiveType::Char => "u8",
        PrimitiveType::Int8 => "i8",
        PrimitiveType::UInt8 => "u8",
        PrimitiveType::Int16 => "i16",
        PrimitiveType::UInt16 => "u16",
        PrimitiveType::Int32 => "i32",
        PrimitiveType::UInt32 => "u32",
        PrimitiveType::Int64 => "i64",
        PrimitiveType::UInt64 => "u64",
        PrimitiveType::Float => "f32",
        PrimitiveType::Double => "f64",
    }
}

struct CompositeMember {
    name: String,
    offset: usize,
    since_version: u16,
    member_type: MemberType,
}

#[derive(Clone)]
enum MemberType {
    Primitive {
        prim: PrimitiveType,
        length: Option<usize>,
        presence: Presence,
        constant_value: Option<String>,
    },
    Composite {
        name: String,
        size: usize,
    },
    Enum {
        name: String,
        encoding_type: PrimitiveType,
    },
    Set {
        name: String,
        encoding_type: PrimitiveType,
    },
}

fn parse_composite_members(tokens: &[Token]) -> Vec<CompositeMember> {
    let mut members = Vec::new();
    let mut i = 1;
    let end_limit = tokens.len() - 1;
    while i < end_limit {
        if tokens[i].signal == Signal::BeginField {
            let name = tokens[i].name.clone();
            let offset = tokens[i].encoding.offset.unwrap_or(0);
            let since_version = tokens[i].encoding.since_version;
            let presence = tokens[i].encoding.presence;
            let constant_value = tokens[i].encoding.constant_value.clone();
            let length = tokens[i].encoding.length;

            let member_type = if i + 2 < tokens.len() && tokens[i + 1].signal == Signal::BeginComposite {
                let comp_name = tokens[i + 1].name.clone();
                let size = tokens[i + 1].encoding.offset.unwrap_or(0);
                MemberType::Composite { name: comp_name, size }
            } else if i + 2 < tokens.len() && tokens[i + 1].signal == Signal::BeginEnum {
                let enum_name = tokens[i + 1].name.clone();
                let encoding_type = tokens[i + 1].encoding.primitive_type.unwrap_or(PrimitiveType::UInt8);
                MemberType::Enum { name: enum_name, encoding_type }
            } else if i + 2 < tokens.len() && tokens[i + 1].signal == Signal::BeginSet {
                let set_name = tokens[i + 1].name.clone();
                let encoding_type = tokens[i + 1].encoding.primitive_type.unwrap_or(PrimitiveType::UInt8);
                MemberType::Set { name: set_name, encoding_type }
            } else {
                let prim = tokens[i].encoding.primitive_type.unwrap_or(PrimitiveType::UInt8);
                MemberType::Primitive {
                    prim,
                    length,
                    presence,
                    constant_value,
                }
            };

            members.push(CompositeMember {
                name,
                offset,
                since_version,
                member_type,
            });

            let end = find_matching_end(tokens, i, Signal::BeginField, Signal::EndField);
            i = end + 1;
        } else {
            i += 1;
        }
    }
    members
}

fn generate_enum(src: &mut String, tokens: &[Token]) {
    let raw_name = &tokens[0].name;
    let name = to_pascal_case(raw_name);
    let encoding_type = tokens[0].encoding.primitive_type.unwrap_or(PrimitiveType::UInt8);
    let r_type = rust_type(encoding_type);
    let is_char = encoding_type == PrimitiveType::Char;

    src.push_str(&format!(
        "#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]\n\
         #[repr(transparent)]\n\
         pub struct {}(pub {});\n\n",
        name, r_type
    ));

    src.push_str(&format!(
        "#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]\n\
         #[repr({})]\n\
         pub enum {}Kind {{\n",
        r_type, name
    ));

    for t in tokens {
        if t.signal == Signal::Encoding {
            if let Some(ref val) = t.encoding.constant_value {
                let variant_name = to_pascal_case(&t.name);
                let disc = format_discriminant(val, is_char);
                src.push_str(&format!("    {} = {},\n", variant_name, disc));
            }
        }
    }
    src.push_str("}\n\n");

    src.push_str(&format!(
        "impl {} {{\n",
        name
    ));

    for t in tokens {
        if t.signal == Signal::Encoding {
            if let Some(ref val) = t.encoding.constant_value {
                let const_name = to_upper_snake_case(&t.name);
                let disc = format_discriminant(val, is_char);
                src.push_str(&format!("    pub const {}: Self = Self({});\n", const_name, disc));
            }
        }
    }

    src.push_str(&format!(
        "\n    pub const fn kind(self) -> Option<{}Kind> {{\n\
                  match self.0 {{\n",
        name
    ));
    for t in tokens {
        if t.signal == Signal::Encoding {
            if let Some(ref val) = t.encoding.constant_value {
                let variant_name = to_pascal_case(&t.name);
                let disc = format_discriminant(val, is_char);
                src.push_str(&format!("            {} => Some({}Kind::{}),\n", disc, name, variant_name));
            }
        }
    }
    src.push_str("            _ => None,\n        }\n    }\n\n");

    src.push_str(&format!(
        "    pub const fn into_kind(self) -> Option<{}Kind> {{\n\
                  self.kind()\n\
              }}\n\n",
        name
    ));

    src.push_str(&format!(
        "    pub const fn raw(self) -> {} {{\n\
                  self.0\n\
              }}\n",
        r_type
    ));
    src.push_str("}\n\n");

    src.push_str(&format!(
        "impl From<{}> for {} {{\n\
              #[inline(always)]\n\
              fn from(val: {}) -> Self {{\n\
                  Self(val)\n\
              }}\n\
          }}\n\n\
          impl From<{}> for {} {{\n\
              #[inline(always)]\n\
              fn from(val: {}) -> Self {{\n\
                  val.0\n\
              }}\n\
          }}\n\n",
        r_type, name, r_type, name, r_type, name
    ));

    src.push_str(&format!(
        "impl TryFrom<{}> for {}Kind {{\n\
             type Error = ();\n\
             #[inline]\n\
             fn try_from(val: {}) -> Result<Self, Self::Error> {{\n\
                 val.kind().ok_or(())\n\
             }}\n\
         }}\n\n",
        name, name, name
    ));
}

fn generate_set(src: &mut String, tokens: &[Token]) {
    let raw_name = &tokens[0].name;
    let name = to_pascal_case(raw_name);
    let encoding_type = tokens[0].encoding.primitive_type.unwrap_or(PrimitiveType::UInt8);
    let r_type = rust_type(encoding_type);

    src.push_str(&format!(
        "#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]\n\
         #[repr(transparent)]\n\
         pub struct {}(pub {});\n\n",
        name, r_type
    ));

    src.push_str(&format!(
        "impl {} {{\n\
             pub const fn raw(self) -> {} {{\n\
                 self.0\n\
             }}\n\n\
             pub const fn default() -> Self {{\n\
                 Self(0)\n\
             }}\n\n",
        name, r_type
    ));

    for t in tokens {
        if t.signal == Signal::Encoding {
            if let Some(ref val) = t.encoding.constant_value {
                let bit_index: u8 = val.parse().unwrap_or(0);
                let bit_name_lower = to_snake_case(&t.name);
                src.push_str(&format!(
                    "    pub const fn {}(self) -> bool {{\n\
                              (self.0 & (1 << {})) != 0\n\
                          }}\n\n\
                          pub fn set_{}(&mut self, val: bool) {{\n\
                              if val {{\n\
                                  self.0 |= 1 << {};\n\
                              }} else {{\n\
                                  self.0 &= !(1 << {});\n\
                              }}\n\
                          }}\n\n",
                    bit_name_lower, bit_index, bit_name_lower, bit_index, bit_index
                ));
            }
        }
    }
    src.push_str("}\n\n");

    src.push_str(&format!(
        "impl From<{}> for {} {{\n\
              #[inline(always)]\n\
              fn from(val: {}) -> Self {{\n\
                  Self(val)\n\
              }}\n\
          }}\n\n\
          impl From<{}> for {} {{\n\
              #[inline(always)]\n\
              fn from(val: {}) -> Self {{\n\
                  val.0\n\
              }}\n\
          }}\n\n",
        r_type, name, r_type, name, r_type, name
    ));
}

fn generate_composite(src: &mut String, tokens: &[Token], byte_order: ByteOrder) {
    let raw_name = &tokens[0].name;
    let name = to_pascal_case(raw_name);
    let size = tokens[0].encoding.offset.unwrap_or(0);

    src.push_str(&format!(
        "#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]\n\
         #[repr(transparent)]\n\
         pub struct {}(pub [u8; {}]);\n\n",
        name, size
    ));

    src.push_str(&format!(
        "impl {} {{\n",
        name
    ));

    let members = parse_composite_members(tokens);
    let order_suffix = match byte_order {
        ByteOrder::LittleEndian => "le",
        ByteOrder::BigEndian => "be",
    };

    // Getters for members
    for m in &members {
        let field_name = to_snake_case(&m.name);
        match &m.member_type {
            MemberType::Primitive { prim, length, presence, constant_value } => {
                let r_type = rust_type(*prim);
                let prim_size = prim.size();
                if *presence == Presence::Constant {
                    if let Some(val) = constant_value {
                        if *prim == PrimitiveType::Char && val.len() > 1 {
                            src.push_str(&format!(
                                "    pub const fn {}(&self) -> &'static str {{\n\
                                         \"{}\"\n\
                                     }}\n\n",
                                field_name, val
                            ));
                        } else {
                            let expr = constant_value_expr(*prim, val);
                            src.push_str(&format!(
                                "    pub const fn {}(&self) -> {} {{\n\
                                         {}\n\
                                     }}\n\n",
                                field_name, r_type, expr
                            ));
                        }
                    }
                } else if let Some(len) = length {
                    src.push_str(&format!(
                        "    pub const fn {}(&self) -> [{}; {}] {{\n\
                                 let mut res = [0 as {}; {}];\n\
                                 let mut idx = 0;\n\
                                 while idx < {} {{\n\
                                     let offset = {} + idx * {};\n\
                                     let mut bytes = [0u8; {}];\n\
                                     let mut j = 0;\n\
                                     while j < {} {{\n\
                                         bytes[j] = self.0[offset + j];\n\
                                         j += 1;\n\
                                     }}\n\
                                     res[idx] = {}::from_{}_bytes(bytes);\n\
                                     idx += 1;\n\
                                 }}\n\
                                 res\n\
                             }}\n\n",
                        field_name, r_type, len, r_type, len, len, m.offset, prim_size, prim_size, prim_size, r_type, order_suffix
                    ));
                } else {
                    src.push_str(&format!(
                        "    pub const fn {}(&self) -> {} {{\n\
                                 let mut bytes = [0u8; {}];\n\
                                 let mut j = 0;\n\
                                 while j < {} {{\n\
                                     bytes[j] = self.0[{} + j];\n\
                                     j += 1;\n\
                                 }}\n\
                                 {}::from_{}_bytes(bytes)\n\
                             }}\n\n",
                        field_name, r_type, prim_size, prim_size, m.offset, r_type, order_suffix
                    ));
                }
            }
            MemberType::Composite { name: comp_name, size: comp_size } => {
                let target_name = to_pascal_case(comp_name);
                src.push_str(&format!(
                    "    pub const fn {}(&self) -> {} {{\n\
                             let mut bytes = [0u8; {}];\n\
                             let mut j = 0;\n\
                             while j < {} {{\n\
                                 bytes[j] = self.0[{} + j];\n\
                                 j += 1;\n\
                             }}\n\
                             {}(bytes)\n\
                         }}\n\n",
                    field_name, target_name, comp_size, comp_size, m.offset, target_name
                ));
            }
            MemberType::Enum { name: enum_name, encoding_type } => {
                let target_name = to_pascal_case(enum_name);
                let r_type = rust_type(*encoding_type);
                let prim_size = encoding_type.size();
                src.push_str(&format!(
                    "    pub const fn {}(&self) -> {} {{\n\
                             let mut bytes = [0u8; {}];\n\
                             let mut j = 0;\n\
                             while j < {} {{\n\
                                 bytes[j] = self.0[{} + j];\n\
                                 j += 1;\n\
                             }}\n\
                             {}({}::from_{}_bytes(bytes))\n\
                         }}\n\n",
                    field_name, target_name, prim_size, prim_size, m.offset, target_name, r_type, order_suffix
                ));
            }
            MemberType::Set { name: set_name, encoding_type } => {
                let target_name = to_pascal_case(set_name);
                let r_type = rust_type(*encoding_type);
                let prim_size = encoding_type.size();
                src.push_str(&format!(
                    "    pub const fn {}(&self) -> {} {{\n\
                             let mut bytes = [0u8; {}];\n\
                             let mut j = 0;\n\
                             while j < {} {{\n\
                                 bytes[j] = self.0[{} + j];\n\
                                 j += 1;\n\
                             }}\n\
                             {}({}::from_{}_bytes(bytes))\n\
                         }}\n\n",
                    field_name, target_name, prim_size, prim_size, m.offset, target_name, r_type, order_suffix
                ));
            }
        }
    }

    // Constructor `new(...)`
    src.push_str("    pub const fn new(");
    let mut params = Vec::new();
    for m in &members {
        let field_name = to_snake_case(&m.name);
        match &m.member_type {
            MemberType::Primitive { prim, length, presence, .. } => {
                if *presence != Presence::Constant {
                    let r_type = rust_type(*prim);
                    if let Some(len) = length {
                        params.push(format!("{}: [{}; {}]", field_name, r_type, len));
                    } else {
                        params.push(format!("{}: {}", field_name, r_type));
                    }
                }
            }
            MemberType::Composite { name: comp_name, .. } => {
                params.push(format!("{}: {}", field_name, to_pascal_case(comp_name)));
            }
            MemberType::Enum { name: enum_name, .. } => {
                params.push(format!("{}: {}", field_name, to_pascal_case(enum_name)));
            }
            MemberType::Set { name: set_name, .. } => {
                params.push(format!("{}: {}", field_name, to_pascal_case(set_name)));
            }
        }
    }
    src.push_str(&params.join(", "));
    src.push_str(") -> Self {\n");
    src.push_str(&format!("        let mut bytes = [0u8; {}];\n", size));

    for m in &members {
        let field_name = to_snake_case(&m.name);
        match &m.member_type {
            MemberType::Primitive { prim, length, presence, constant_value } => {
                let prim_size = prim.size();
                if *presence == Presence::Constant {
                    if let Some(val) = constant_value {
                        if *prim == PrimitiveType::Char && val.len() > 1 {
                            src.push_str(&format!(
                                "        let const_bytes = b\"{}\";\n\
                                 let mut j = 0;\n\
                                 while j < const_bytes.len() {{\n\
                                     bytes[{} + j] = const_bytes[j];\n\
                                     j += 1;\n\
                                 }}\n",
                                val, m.offset
                            ));
                        } else {
                            let expr = constant_value_expr(*prim, val);
                            src.push_str(&format!(
                                "        let const_val = {};\n\
                                 let const_bytes = const_val.to_{}_bytes();\n\
                                 let mut j = 0;\n\
                                 while j < {} {{\n\
                                     bytes[{} + j] = const_bytes[j];\n\
                                     j += 1;\n\
                                 }}\n",
                                expr, order_suffix, prim_size, m.offset
                            ));
                        }
                    }
                } else if let Some(len) = length {
                    src.push_str(&format!(
                        "        let mut idx = 0;\n\
                                 while idx < {} {{\n\
                                     let val_bytes = {}[idx].to_{}_bytes();\n\
                                     let mut j = 0;\n\
                                     while j < {} {{\n\
                                         bytes[{} + idx * {} + j] = val_bytes[j];\n\
                                         j += 1;\n\
                                     }}\n\
                                     idx += 1;\n\
                                 }}\n",
                        len, field_name, order_suffix, prim_size, m.offset, prim_size
                    ));
                } else {
                    src.push_str(&format!(
                        "        let val_bytes = {}.to_{}_bytes();\n\
                                 let mut j = 0;\n\
                                 while j < {} {{\n\
                                     bytes[{} + j] = val_bytes[j];\n\
                                     j += 1;\n\
                                 }}\n",
                        field_name, order_suffix, prim_size, m.offset
                    ));
                }
            }
            MemberType::Composite { size: comp_size, .. } => {
                src.push_str(&format!(
                    "        let mut j = 0;\n\
                             while j < {} {{\n\
                                 bytes[{} + j] = {}.0[j];\n\
                                 j += 1;\n\
                             }}\n",
                    comp_size, m.offset, field_name
                ));
            }
            MemberType::Enum { encoding_type, .. } => {
                let prim_size = encoding_type.size();
                src.push_str(&format!(
                    "        let val_bytes = {}.0.to_{}_bytes();\n\
                             let mut j = 0;\n\
                             while j < {} {{\n\
                                 bytes[{} + j] = val_bytes[j];\n\
                                 j += 1;\n\
                             }}\n",
                    field_name, order_suffix, prim_size, m.offset
                ));
            }
            MemberType::Set { encoding_type, .. } => {
                let prim_size = encoding_type.size();
                src.push_str(&format!(
                    "        let val_bytes = {}.0.to_{}_bytes();\n\
                             let mut j = 0;\n\
                             while j < {} {{\n\
                                 bytes[{} + j] = val_bytes[j];\n\
                                 j += 1;\n\
                             }}\n",
                    field_name, order_suffix, prim_size, m.offset
                ));
            }
        }
    }

    src.push_str("        Self(bytes)\n    }\n");
    src.push_str("}\n\n");
}

fn get_dimension_info(elements: &SchemaElements, dim_type: &str) -> (String, usize, String, String) {
    let raw_name = dim_type;
    let name = to_pascal_case(raw_name);
    let mut size = 4;
    let mut bl = "block_length".to_string();
    let mut num = "num_in_group".to_string();
    if let Some(comp) = elements.composites.iter().find(|c| c[0].name == raw_name) {
        size = comp[0].encoding.offset.unwrap_or(4);
        let members = parse_composite_members(comp);
        for m in members {
            let lower = m.name.to_lowercase();
            if lower.contains("blocklength") {
                bl = to_snake_case(&m.name);
            } else if lower.contains("numingroup") || lower.contains("count") {
                num = to_snake_case(&m.name);
            }
        }
    }
    (name, size, bl, num)
}

fn get_vardata_info(elements: &SchemaElements, type_name: &str) -> (String, usize, String, PrimitiveType) {
    let raw_name = type_name;
    let name = to_pascal_case(raw_name);
    let mut size = 4;
    let mut len_field = "length".to_string();
    let mut prim = PrimitiveType::UInt32;
    if let Some(comp) = elements.composites.iter().find(|c| c[0].name == raw_name) {
        let members = parse_composite_members(comp);
        for m in members {
            if m.name == "length" {
                len_field = to_snake_case(&m.name);
                if let MemberType::Primitive { prim: p, .. } = m.member_type {
                    prim = p;
                }
            }
            if m.name == "varData" {
                size = m.offset;
            }
        }
    }
    (name, size, len_field, prim)
}

fn generate_dim_new_call(elements: &SchemaElements, dim_type: &str, block_len_expr: &str, count_expr: &str) -> String {
    let raw_name = dim_type;
    let name = to_pascal_case(raw_name);
    let mut args = vec![block_len_expr.to_string(), count_expr.to_string()];
    if let Some(comp) = elements.composites.iter().find(|c| c[0].name == raw_name) {
        let members = parse_composite_members(comp);
        if members.len() == 2 {
            let lower_0 = members[0].name.to_lowercase();
            if lower_0.contains("numingroup") || lower_0.contains("count") {
                args = vec![count_expr.to_string(), block_len_expr.to_string()];
            }
        }
    }
    format!("{}::new({})", name, args.join(", "))
}

fn generate_message_decoder(
    src: &mut String,
    msg: &MessageStructure,
    elements: &SchemaElements,
    byte_order: ByteOrder,
    schema_id: u16,
    schema_version: u16,
    header_type: &str,
) {
    let raw_name = &msg.name;
    let name = to_pascal_case(raw_name);
    let order_suffix = match byte_order {
        ByteOrder::LittleEndian => "le",
        ByteOrder::BigEndian => "be",
    };

    let block_length = msg.fields.iter().fold(0, |acc, f| {
        let size = match f.field_type {
            FieldType::Primitive(p, length) => p.size() * length.unwrap_or(1),
            FieldType::Composite { size, .. } => size,
            FieldType::Enum { encoding_type, .. } => encoding_type.size(),
            FieldType::Set { encoding_type, .. } => encoding_type.size(),
        };
        acc.max(f.offset + size)
    });

    let header_pascal = to_pascal_case(header_type);
    let (header_bl, header_ti, header_si, header_vr) = {
        let mut bl = "block_length".to_string();
        let mut ti = "template_id".to_string();
        let mut si = "schema_id".to_string();
        let mut vr = "version".to_string();
        if let Some(comp) = elements.composites.iter().find(|c| c[0].name == header_type) {
            let members = parse_composite_members(comp);
            for m in members {
                let lower = m.name.to_lowercase();
                if lower.contains("blocklength") {
                    bl = to_snake_case(&m.name);
                } else if lower.contains("templateid") {
                    ti = to_snake_case(&m.name);
                } else if lower.contains("schemaid") {
                    si = to_snake_case(&m.name);
                } else if lower.contains("version") {
                    vr = to_snake_case(&m.name);
                }
            }
        }
        (bl, ti, si, vr)
    };

    let header_size = elements.composites.iter()
        .find(|c| c[0].name == header_type)
        .and_then(|c| c[0].encoding.offset)
        .unwrap_or(8);

    // 1. Decoder Struct
    src.push_str(&format!(
        "#[derive(Clone, Copy)]\n\
         pub struct {}Decoder<'a> {{\n\
             buf: &'a [u8],\n\
             pos: usize,\n\
             acting_version: u16,\n\
             acting_block_length: usize,\n\
         }}\n\n",
        name
    ));

    src.push_str(&format!(
        "impl<'a> {}Decoder<'a> {{\n\
             pub const SCHEMA_ID: u16 = {};\n\
             pub const SCHEMA_VERSION: u16 = {};\n\
             pub const TEMPLATE_ID: u16 = {};\n\
             pub const BLOCK_LENGTH: usize = {};\n\n",
        name, schema_id, schema_version, msg.id, block_length
    ));

    src.push_str(&format!(
        "    pub const fn wrap(buf: &'a [u8], pos: usize, acting_block_length: usize, acting_version: u16) -> Self {{\n\
                 Self {{\n\
                     buf,\n\
                     pos,\n\
                     acting_block_length,\n\
                     acting_version,\n\
                 }}\n\
             }}\n\n\
             pub const fn wrap_and_apply_header(buf: &'a [u8], pos: usize) -> Result<Self, sbe_rt::DecodeError> {{\n\
                 if pos + {} > buf.len() {{\n\
                     return Err(sbe_rt::DecodeError::BufferTooShort {{ needed: pos + {}, available: buf.len() }});\n\
                 }}\n\
                 let mut header_bytes = [0u8; {}];\n\
                 let mut j = 0;\n\
                 while j < {} {{\n\
                     header_bytes[j] = buf[pos + j];\n\
                     j += 1;\n\
                 }}\n\
                 let header = {}(header_bytes);\n\
                 if header.{}() != Self::SCHEMA_ID {{\n\
                     return Err(sbe_rt::DecodeError::WrongSchema {{ expected: Self::SCHEMA_ID, actual: header.{}() }});\n\
                 }}\n\
                 Ok(Self::wrap(buf, pos + {}, header.{}() as usize, header.{}()))\n\
             }}\n\n",
        header_size, header_size, header_size, header_size, header_pascal, header_si, header_si, header_size, header_bl, header_vr
    ));

    src.push_str("    pub const fn acting_version(&self) -> u16 {\n        self.acting_version\n    }\n\n");
    src.push_str("    pub const fn acting_block_length(&self) -> usize {\n        self.acting_block_length\n    }\n\n");

    // Fields Getters
    for f in &msg.fields {
        let f_name = to_snake_case(&f.name);
        let offset = f.offset;
        let since = f.since_version;

        match &f.field_type {
            FieldType::Primitive(prim, length) => {
                let r_type = rust_type(*prim);
                let prim_size = prim.size();

                if f.presence == Presence::Constant {
                    if let Some(ref val) = f.constant_value {
                        if *prim == PrimitiveType::Char && val.len() > 1 {
                            src.push_str(&format!(
                                "    pub const fn {}(&self) -> &'static str {{\n\
                                         \"{}\"\n\
                                     }}\n\n",
                                f_name, val
                            ));
                        } else {
                            let expr = constant_value_expr(*prim, val);
                            src.push_str(&format!(
                                "    pub const fn {}(&self) -> {} {{\n\
                                         {}\n\
                                     }}\n\n",
                                f_name, r_type, expr
                            ));
                        }
                    }
                } else if let Some(len) = length {
                    src.push_str(&format!(
                        "    pub const fn {}(&self) -> Result<[{}; {}], sbe_rt::DecodeError> {{\n\
                                 if self.acting_version < {} || {} > self.acting_block_length {{\n\
                                     return Ok([0 as {}; {}]);\n\
                                 }}\n\
                                 let offset = self.pos + {};\n\
                                 let size = {};\n\
                                 if offset + size > self.buf.len() {{\n\
                                     return Err(sbe_rt::DecodeError::BufferTooShort {{ needed: offset + size, available: self.buf.len() }});\n\
                                 }}\n\
                                 let mut res = [0 as {}; {}];\n\
                                 let mut idx = 0;\n\
                                 while idx < {} {{\n\
                                     let offset = self.pos + {} + idx * {};\n\
                                     let mut bytes = [0u8; {}];\n\
                                     let mut j = 0;\n\
                                     while j < {} {{\n\
                                         bytes[j] = self.buf[offset + j];\n\
                                         j += 1;\n\
                                     }}\n\
                                     res[idx] = {}::from_{}_bytes(bytes);\n\
                                     idx += 1;\n\
                                 }}\n\
                                 Ok(res)\n\
                             }}\n\n",
                        f_name, r_type, len, since, offset + prim_size * len, r_type, len, offset, prim_size * len, r_type, len, len, offset, prim_size, prim_size, prim_size, r_type, order_suffix
                    ));

                    src.push_str(&format!(
                        "    pub const unsafe fn {}_unchecked(&self) -> [{}; {}] {{\n\
                                 let offset = self.pos + {};\n\
                                 let mut res = [0 as {}; {}];\n\
                                 let mut idx = 0;\n\
                                 while idx < {} {{\n\
                                     let offset = self.pos + {} + idx * {};\n\
                                     let mut bytes = [0u8; {}];\n\
                                     let mut j = 0;\n\
                                     while j < {} {{\n\
                                         bytes[j] = *self.buf.as_ptr().add(offset + j);\n\
                                         j += 1;\n\
                                     }}\n\
                                     res[idx] = {}::from_{}_bytes(bytes);\n\
                                     idx += 1;\n\
                                 }}\n\
                                 res\n\
                             }}\n\n",
                        f_name, r_type, len, offset, r_type, len, len, offset, prim_size, prim_size, prim_size, r_type, order_suffix
                    ));

                    if since == 0 {
                        src.push_str(&format!(
                            "    pub const fn raw_{}(&self) -> [{}; {}] {{\n\
                                     #[allow(unused_unsafe)]\n\
                                     unsafe {{ self.{}_unchecked() }}\n\
                                 }}\n\n",
                            f_name, r_type, len, f_name
                        ));
                    } else {
                        src.push_str(&format!(
                            "    pub const fn raw_{}(&self) -> Option<[{}; {}]> {{\n\
                                     if self.acting_version < {} || {} > self.acting_block_length {{\n\
                                         return None;\n\
                                     }}\n\
                                     #[allow(unused_unsafe)]\n\
                                     Some(unsafe {{ self.{}_unchecked() }})\n\
                                 }}\n\n",
                            f_name, r_type, len, since, offset + prim_size * len, f_name
                        ));
                    }
                } else {
                    if f.presence == Presence::Optional {
                        let null_val = f.null_value.unwrap_or(0);
                        let null_check = if *prim == PrimitiveType::Float {
                            format!("val.to_bits() == {} as u32", null_val)
                        } else if *prim == PrimitiveType::Double {
                            format!("val.to_bits() == {}", null_val)
                        } else {
                            format!("val == {} as {}", null_val, r_type)
                        };

                        src.push_str(&format!(
                            "    pub const fn {}(&self) -> Result<Option<{}>, sbe_rt::DecodeError> {{\n\
                                     if self.acting_version < {} || {} > self.acting_block_length {{\n\
                                         return Ok(None);\n\
                                     }}\n\
                                     let offset = self.pos + {};\n\
                                     if offset + {} > self.buf.len() {{\n\
                                         return Err(sbe_rt::DecodeError::BufferTooShort {{ needed: offset + {}, available: self.buf.len() }});\n\
                                     }}\n\
                                     let mut bytes = [0u8; {}];\n\
                                     let mut j = 0;\n\
                                     while j < {} {{\n\
                                         bytes[j] = self.buf[offset + j];\n\
                                         j += 1;\n\
                                     }}\n\
                                     let val = {}::from_{}_bytes(bytes);\n\
                                     if {} {{\n\
                                         Ok(None)\n\
                                     }} else {{\n\
                                         Ok(Some(val))\n\
                                     }}\n\
                                 }}\n\n",
                            f_name, r_type, since, offset + prim_size, offset, prim_size, prim_size, prim_size, prim_size, r_type, order_suffix, null_check
                        ));
                    } else if since > 0 {
                        src.push_str(&format!(
                            "    pub const fn {}(&self) -> Result<Option<{}>, sbe_rt::DecodeError> {{\n\
                                     if self.acting_version < {} || {} > self.acting_block_length {{\n\
                                         return Ok(None);\n\
                                     }}\n\
                                     let offset = self.pos + {};\n\
                                     if offset + {} > self.buf.len() {{\n\
                                         return Err(sbe_rt::DecodeError::BufferTooShort {{ needed: offset + {}, available: self.buf.len() }});\n\
                                     }}\n\
                                     let mut bytes = [0u8; {}];\n\
                                     let mut j = 0;\n\
                                     while j < {} {{\n\
                                         bytes[j] = self.buf[offset + j];\n\
                                         j += 1;\n\
                                     }}\n\
                                     Ok(Some({}::from_{}_bytes(bytes)))\n\
                                 }}\n\n",
                            f_name, r_type, since, offset + prim_size, offset, prim_size, prim_size, prim_size, prim_size, r_type, order_suffix
                        ));
                    } else {
                        src.push_str(&format!(
                            "    pub const fn {}(&self) -> Result<{}, sbe_rt::DecodeError> {{\n\
                                     let offset = self.pos + {};\n\
                                     if offset + {} > self.buf.len() {{\n\
                                         return Err(sbe_rt::DecodeError::BufferTooShort {{ needed: offset + {}, available: self.buf.len() }});\n\
                                     }}\n\
                                     let mut bytes = [0u8; {}];\n\
                                     let mut j = 0;\n\
                                     while j < {} {{\n\
                                         bytes[j] = self.buf[offset + j];\n\
                                         j += 1;\n\
                                     }}\n\
                                     Ok({}::from_{}_bytes(bytes))\n\
                                 }}\n\n",
                            f_name, r_type, offset, prim_size, prim_size, prim_size, prim_size, r_type, order_suffix
                        ));
                    }

                    src.push_str(&format!(
                        "    pub const unsafe fn {}_unchecked(&self) -> {} {{\n\
                                 let offset = self.pos + {};\n\
                                 let mut bytes = [0u8; {}];\n\
                                 let mut j = 0;\n\
                                 while j < {} {{\n\
                                     bytes[j] = *self.buf.as_ptr().add(offset + j);\n\
                                     j += 1;\n\
                                 }}\n\
                                 {}::from_{}_bytes(bytes)\n\
                             }}\n\n",
                        f_name, r_type, offset, prim_size, prim_size, r_type, order_suffix
                    ));

                    if since == 0 {
                        src.push_str(&format!(
                            "    pub const fn raw_{}(&self) -> {} {{\n\
                                     #[allow(unused_unsafe)]\n\
                                     unsafe {{ self.{}_unchecked() }}\n\
                                 }}\n\n",
                            f_name, r_type, f_name
                        ));
                    } else {
                        src.push_str(&format!(
                            "    pub const fn raw_{}(&self) -> Option<{}> {{\n\
                                     if self.acting_version < {} || {} > self.acting_block_length {{\n\
                                         return None;\n\
                                     }}\n\
                                     #[allow(unused_unsafe)]\n\
                                     Some(unsafe {{ self.{}_unchecked() }})\n\
                                 }}\n\n",
                            f_name, r_type, since, offset + prim_size, f_name
                        ));
                    }
                }
            }
            FieldType::Composite { name: comp_name, size: comp_size } => {
                let target_name = to_pascal_case(comp_name);
                src.push_str(&format!(
                    "    pub const fn {}(&self) -> Result<{}, sbe_rt::DecodeError> {{\n\
                             if self.acting_version < {} || {} > self.acting_block_length {{\n\
                                 return Ok({}([0u8; {}]));\n\
                             }}\n\
                             let offset = self.pos + {};\n\
                             if offset + {} > self.buf.len() {{\n\
                                 return Err(sbe_rt::DecodeError::BufferTooShort {{ needed: offset + {}, available: self.buf.len() }});\n\
                             }}\n\
                             let mut bytes = [0u8; {}];\n\
                             let mut j = 0;\n\
                             while j < {} {{\n\
                                 bytes[j] = self.buf[offset + j];\n\
                                 j += 1;\n\
                             }}\n\
                             Ok({}(bytes))\n\
                         }}\n\n",
                    f_name, target_name, since, offset + comp_size, target_name, comp_size, offset, comp_size, comp_size, comp_size, comp_size, target_name
                ));

                src.push_str(&format!(
                    "    pub const unsafe fn {}_unchecked(&self) -> {} {{\n\
                             let offset = self.pos + {};\n\
                             let mut bytes = [0u8; {}];\n\
                             let mut j = 0;\n\
                             while j < {} {{\n\
                                 bytes[j] = *self.buf.as_ptr().add(offset + j);\n\
                                 j += 1;\n\
                             }}\n\
                             {}(bytes)\n\
                         }}\n\n",
                    f_name, target_name, offset, comp_size, comp_size, target_name
                ));
            }
            FieldType::Enum { name: enum_name, encoding_type } => {
                let target_name = to_pascal_case(enum_name);
                let r_type = rust_type(*encoding_type);
                let prim_size = encoding_type.size();

                src.push_str(&format!(
                    "    pub const fn {}(&self) -> Result<{}, sbe_rt::DecodeError> {{\n\
                             if self.acting_version < {} || {} > self.acting_block_length {{\n\
                                 return Ok({}(0 as {}));\n\
                             }}\n\
                             let offset = self.pos + {};\n\
                             if offset + {} > self.buf.len() {{\n\
                                 return Err(sbe_rt::DecodeError::BufferTooShort {{ needed: offset + {}, available: self.buf.len() }});\n\
                             }}\n\
                             let mut bytes = [0u8; {}];\n\
                             let mut j = 0;\n\
                             while j < {} {{\n\
                                 bytes[j] = self.buf[offset + j];\n\
                                 j += 1;\n\
                             }}\n\
                             Ok({}({}::from_{}_bytes(bytes)))\n\
                         }}\n\n",
                    f_name, target_name, since, offset + prim_size, target_name, r_type, offset, prim_size, prim_size, prim_size, prim_size, target_name, r_type, order_suffix
                ));

                src.push_str(&format!(
                    "    pub const unsafe fn {}_unchecked(&self) -> {} {{\n\
                             let offset = self.pos + {};\n\
                             let mut bytes = [0u8; {}];\n\
                             let mut j = 0;\n\
                             while j < {} {{\n\
                                 bytes[j] = *self.buf.as_ptr().add(offset + j);\n\
                                 j += 1;\n\
                             }}\n\
                             {}({}::from_{}_bytes(bytes))\n\
                         }}\n\n",
                    f_name, target_name, offset, prim_size, prim_size, target_name, r_type, order_suffix
                ));
            }
            FieldType::Set { name: set_name, encoding_type } => {
                let target_name = to_pascal_case(set_name);
                let r_type = rust_type(*encoding_type);
                let prim_size = encoding_type.size();

                src.push_str(&format!(
                    "    pub const fn {}(&self) -> Result<{}, sbe_rt::DecodeError> {{\n\
                             if self.acting_version < {} || {} > self.acting_block_length {{\n\
                                 return Ok({}(0 as {}));\n\
                             }}\n\
                             let offset = self.pos + {};\n\
                             if offset + {} > self.buf.len() {{\n\
                                 return Err(sbe_rt::DecodeError::BufferTooShort {{ needed: offset + {}, available: self.buf.len() }});\n\
                             }}\n\
                             let mut bytes = [0u8; {}];\n\
                             let mut j = 0;\n\
                             while j < {} {{\n\
                                 bytes[j] = self.buf[offset + j];\n\
                                 j += 1;\n\
                             }}\n\
                             Ok({}({}::from_{}_bytes(bytes)))\n\
                         }}\n\n",
                    f_name, target_name, since, offset + prim_size, target_name, r_type, offset, prim_size, prim_size, prim_size, prim_size, target_name, r_type, order_suffix
                ));

                src.push_str(&format!(
                    "    pub const unsafe fn {}_unchecked(&self) -> {} {{\n\
                             let offset = self.pos + {};\n\
                             let mut bytes = [0u8; {}];\n\
                             let mut j = 0;\n\
                             while j < {} {{\n\
                                 bytes[j] = *self.buf.as_ptr().add(offset + j);\n\
                                 j += 1;\n\
                             }}\n\
                             {}({}::from_{}_bytes(bytes))\n\
                         }}\n\n",
                    f_name, target_name, offset, prim_size, prim_size, target_name, r_type, order_suffix
                ));
            }
        }
    }

    // Tail Offsets Helpers
    let total_tail = msg.groups.len() + msg.var_data.len();
    src.push_str(&format!(
        "    #[inline]\n\
             fn tail_offset_0(&self) -> Result<usize, sbe_rt::DecodeError> {{\n\
                 Ok(self.pos + self.acting_block_length)\n\
             }}\n\n"
    ));

    let mut k = 0;
    for g in &msg.groups {
        let (dim_name, dim_size, bl_field, count_field) = get_dimension_info(elements, &g.dimension_type);
        let g_pascal = to_pascal_case(&g.name);
        src.push_str(&format!(
            "    #[inline]\n\
                 fn tail_offset_{}(&self) -> Result<usize, sbe_rt::DecodeError> {{\n\
                     let start = self.tail_offset_{}()?;\n\
                     if start + {} > self.buf.len() {{\n\
                         return Err(sbe_rt::DecodeError::BufferTooShort {{ needed: start + {}, available: self.buf.len() }});\n\
                     }}\n\
                     let mut bytes = [0u8; {}];\n\
                     let mut j = 0;\n\
                     while j < {} {{\n\
                         bytes[j] = self.buf[start + j];\n\
                         j += 1;\n\
                     }}\n\
                     let header = {}(bytes);\n\
                     let count = header.{}() as usize;\n\
                     let block_len = header.{}() as usize;\n\
                     let mut pos = start + {};\n\
                     let mut idx = 0;\n\
                     while idx < count {{\n\
                         pos = {}EntryDecoder::skip(self.buf, pos, block_len, self.acting_version)?;\n\
                         idx += 1;\n\
                     }}\n\
                     Ok(pos)\n\
                 }}\n\n",
            k + 1, k, dim_size, dim_size, dim_size, dim_size, dim_name, count_field, bl_field, dim_size, g_pascal
        ));
        k += 1;
    }

    for vd in &msg.var_data {
        let (type_pascal, prefix_size, len_field, _) = get_vardata_info(elements, &vd.type_name);
        src.push_str(&format!(
            "    #[inline]\n\
                 fn tail_offset_{}(&self) -> Result<usize, sbe_rt::DecodeError> {{\n\
                     let start = self.tail_offset_{}()?;\n\
                     if start + {} > self.buf.len() {{\n\
                         return Err(sbe_rt::DecodeError::BufferTooShort {{ needed: start + {}, available: self.buf.len() }});\n\
                     }}\n\
                     let mut bytes = [0u8; {}];\n\
                     let mut j = 0;\n\
                     while j < {} {{\n\
                         bytes[j] = self.buf[start + j];\n\
                         j += 1;\n\
                     }}\n\
                     let header = {}(bytes);\n\
                     let len = header.{}() as usize;\n\
                     if start + {} + len > self.buf.len() {{\n\
                         return Err(sbe_rt::DecodeError::BufferTooShort {{ needed: start + {} + len, available: self.buf.len() }});\n\
                     }}\n\
                     Ok(start + {} + len)\n\
                 }}\n\n",
            k + 1, k, prefix_size, prefix_size, prefix_size, prefix_size, type_pascal, len_field, prefix_size, prefix_size, prefix_size
        ));
        k += 1;
    }

    // Accessors for Groups
    let mut g_idx = 0;
    for g in &msg.groups {
        let g_pascal = to_pascal_case(&g.name);
        let g_snake = to_snake_case(&g.name);
        src.push_str(&format!(
            "    pub fn {}(&self) -> Result<{}Decoder<'a>, sbe_rt::DecodeError> {{\n\
                     let offset = self.tail_offset_{}()?;\n\
                     {}Decoder::wrap(self.buf, offset, self.acting_version)\n\
                 }}\n\n",
            g_snake, g_pascal, g_idx, g_pascal
        ));
        g_idx += 1;
    }

    // Accessors for VarData
    let mut vd_idx = msg.groups.len();
    for vd in &msg.var_data {
        let (type_pascal, prefix_size, len_field, _) = get_vardata_info(elements, &vd.type_name);
        let vd_snake = to_snake_case(&vd.name);
        src.push_str(&format!(
            "    pub fn {}(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {{\n\
                     let offset = self.tail_offset_{}()?;\n\
                     let mut bytes = [0u8; {}];\n\
                     let mut j = 0;\n\
                     while j < {} {{\n\
                         bytes[j] = self.buf[offset + j];\n\
                         j += 1;\n\
                     }}\n\
                     let header = {}(bytes);\n\
                     let len = header.{}() as usize;\n\
                     let data_offset = offset + {};\n\
                     Ok(&self.buf[data_offset .. data_offset + len])\n\
                 }}\n\n",
            vd_snake, vd_idx, prefix_size, prefix_size, type_pascal, len_field, prefix_size
        ));

        // UTF-8 str accessor
        src.push_str(&format!(
            "    pub fn {}_as_str(&self) -> Result<&'a str, sbe_rt::DecodeError> {{\n\
                     let bytes = self.{}()?;\n\
                     core::str::from_utf8(bytes).map_err(|e| sbe_rt::DecodeError::Utf8(e))\n\
                 }}\n\n",
            vd_snake, vd_snake
        ));

        vd_idx += 1;
    }

    // Message size/as_bytes
    src.push_str(&format!(
        "    pub fn encoded_length(&self) -> Result<usize, sbe_rt::DecodeError> {{\n\
                 let end = self.tail_offset_{}()?;\n\
                 Ok(end - self.pos)\n\
             }}\n\n\
             pub fn encoded_length_with_header(&self) -> Result<usize, sbe_rt::DecodeError> {{\n\
                 let len = self.encoded_length()?;\n\
                 Ok(len + {})\n\
             }}\n\n\
             pub fn as_bytes(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {{\n\
                 let len = self.encoded_length_with_header()?;\n\
                 let start = self.pos - {};\n\
                 Ok(&self.buf[start .. start + len])\n\
             }}\n",
        total_tail, header_size, header_size
    ));

    src.push_str(&format!(
        "}}\n\n\
         impl<'a> sbe_rt::private::Sealed for {}Decoder<'a> {{}}\n\n\
         impl<'a> sbe_rt::SbeMessage for {}Decoder<'a> {{\n\
             const TEMPLATE_ID: u16 = {};\n\
             const BLOCK_LENGTH: usize = {};\n\
             const SCHEMA_ID: u16 = {};\n\
             const SCHEMA_VERSION: u16 = {};\n\
         }}\n\n\
         impl<'a> AsRef<[u8]> for {}Decoder<'a> {{\n\
             fn as_ref(&self) -> &[u8] {{\n\
                 self.as_bytes().unwrap_or(&[])\n\
             }}\n\
         }}\n\n\
         impl<'a> {}Decoder<'a> {{\n\
             pub fn as_ref_opt(&self) -> Option<&[u8]> {{\n\
                 self.as_bytes().ok()\n\
             }}\n\
         }}\n\n",
        name, name, msg.id, block_length, schema_id, schema_version, name, name
    ));

    // Recursively generate Repeating Groups decoders for this message
    for g in &msg.groups {
        generate_group_decoder(src, g, elements, byte_order);
    }
}

fn generate_group_decoder(src: &mut String, g: &MessageGroup, elements: &SchemaElements, byte_order: ByteOrder) {
    let name = to_pascal_case(&g.name);
    let (dim_name, dim_size, bl_field, count_field) = get_dimension_info(elements, &g.dimension_type);
    let order_suffix = match byte_order {
        ByteOrder::LittleEndian => "le",
        ByteOrder::BigEndian => "be",
    };

    src.push_str(&format!(
        "pub struct {}Decoder<'a> {{\n\
             buf: &'a [u8],\n\
             pos: usize,\n\
             count: usize,\n\
             acting_version: u16,\n\
         }}\n\n\
         impl<'a> {}Decoder<'a> {{\n\
             pub const ENTRY_BLOCK_LENGTH: usize = {};\n\n\
             pub fn wrap(buf: &'a [u8], pos: usize, acting_version: u16) -> Result<Self, sbe_rt::DecodeError> {{\n\
                 if pos + {} > buf.len() {{\n\
                     return Err(sbe_rt::DecodeError::BufferTooShort {{ needed: pos + {}, available: buf.len() }});\n\
                 }}\n\
                 let mut bytes = [0u8; {}];\n\
                 let mut j = 0;\n\
                 while j < {} {{\n\
                     bytes[j] = buf[pos + j];\n\
                     j += 1;\n\
                 }}\n\
                 let header = {}(bytes);\n\
                 let count = header.{}() as usize;\n\
                 Ok(Self {{\n\
                     buf,\n\
                     pos: pos + {},\n\
                     count,\n\
                     acting_version,\n\
                 }})\n\
             }}\n\n\
             pub fn is_empty(&self) -> bool {{\n\
                 self.count == 0\n\
             }}\n\n",
        name, name, g.block_length, dim_size, dim_size, dim_size, dim_size, dim_name, count_field, dim_size
    ));

    // Expose fast-path as_chunks if entry has no tail
    let total_tail = g.groups.len() + g.var_data.len();
    if total_tail == 0 {
        src.push_str(&format!(
            "    pub fn as_chunks(&self) -> Result<&'a [[u8; {}]], sbe_rt::DecodeError> {{\n\
                     let len = self.count * {};\n\
                     if self.pos + len > self.buf.len() {{\n\
                         return Err(sbe_rt::DecodeError::BufferTooShort {{ needed: self.pos + len, available: self.buf.len() }});\n\
                     }}\n\
                     let bytes = &self.buf[self.pos .. self.pos + len];\n\
                     let (chunks, _) = bytes.as_chunks::<{}>();\n\
                     Ok(chunks)\n\
                 }}\n\n",
            g.block_length, g.block_length, g.block_length
        ));
    }
    src.push_str("}\n\n");

    // Iterator implementation
    src.push_str(&format!(
        "impl<'a> Iterator for {}Decoder<'a> {{\n\
             type Item = {}EntryDecoder<'a>;\n\
             fn next(&mut self) -> Option<Self::Item> {{\n\
                 if self.count == 0 {{\n\
                     return None;\n\
                 }}\n\
                 let entry = {}EntryDecoder::wrap(self.buf, self.pos, self.acting_version);\n\
                 let size = entry.encoded_length();\n\
                 self.pos += size;\n\
                 self.count -= 1;\n\
                 Some(entry)\n\
             }}\n\
         }}\n\n\
         impl<'a> ExactSizeIterator for {}Decoder<'a> {{\n\
             fn len(&self) -> usize {{\n\
                 self.count\n\
             }}\n\
         }}\n\n",
        name, name, name, name
    ));

    // Entry Decoder Struct
    src.push_str(&format!(
        "pub struct {}EntryDecoder<'a> {{\n\
             buf: &'a [u8],\n\
             pos: usize,\n\
             acting_version: u16,\n\
         }}\n\n\
         impl<'a> {}EntryDecoder<'a> {{\n\
             pub const ENTRY_BLOCK_LENGTH: usize = {};\n\n\
             pub const fn wrap(buf: &'a [u8], pos: usize, acting_version: u16) -> Self {{\n\
                 Self {{ buf, pos, acting_version }}\n\
             }}\n\n",
        name, name, g.block_length
    ));

    // Fields of group entry
    for f in &g.fields {
        let f_name = to_snake_case(&f.name);
        let offset = f.offset;
        let since = f.since_version;

        match &f.field_type {
            FieldType::Primitive(prim, length) => {
                let r_type = rust_type(*prim);
                let prim_size = prim.size();

                if f.presence == Presence::Constant {
                    if let Some(ref val) = f.constant_value {
                        if *prim == PrimitiveType::Char && val.len() > 1 {
                            src.push_str(&format!(
                                "    pub const fn {}(&self) -> &'static str {{\n\
                                         \"{}\"\n\
                                     }}\n\n",
                                f_name, val
                            ));
                        } else {
                            let expr = constant_value_expr(*prim, val);
                            src.push_str(&format!(
                                "    pub const fn {}(&self) -> {} {{\n\
                                         {}\n\
                                     }}\n\n",
                                f_name, r_type, expr
                            ));
                        }
                    }
                } else if let Some(len) = length {
                    src.push_str(&format!(
                        "    pub const fn {}(&self) -> Result<[{}; {}], sbe_rt::DecodeError> {{\n\
                                 let offset = self.pos + {};\n\
                                 let size = {};\n\
                                 if offset + size > self.buf.len() {{\n\
                                     return Err(sbe_rt::DecodeError::BufferTooShort {{ needed: offset + size, available: self.buf.len() }});\n\
                                 }}\n\
                                 let mut res = [0 as {}; {}];\n\
                                 let mut idx = 0;\n\
                                 while idx < {} {{\n\
                                     let offset = self.pos + {} + idx * {};\n\
                                     let mut bytes = [0u8; {}];\n\
                                     let mut j = 0;\n\
                                     while j < {} {{\n\
                                         bytes[j] = self.buf[offset + j];\n\
                                         j += 1;\n\
                                     }}\n\
                                     res[idx] = {}::from_{}_bytes(bytes);\n\
                                     idx += 1;\n\
                                 }}\n\
                                 Ok(res)\n\
                             }}\n\n",
                        f_name, r_type, len, offset, prim_size * len, r_type, len, len, offset, prim_size, prim_size, prim_size, r_type, order_suffix
                    ));

                    src.push_str(&format!(
                        "    pub const unsafe fn {}_unchecked(&self) -> [{}; {}] {{\n\
                                 let offset = self.pos + {};\n\
                                 let mut res = [0 as {}; {}];\n\
                                 let mut idx = 0;\n\
                                 while idx < {} {{\n\
                                     let offset = self.pos + {} + idx * {};\n\
                                     let mut bytes = [0u8; {}];\n\
                                     let mut j = 0;\n\
                                     while j < {} {{\n\
                                         bytes[j] = *self.buf.as_ptr().add(offset + j);\n\
                                         j += 1;\n\
                                     }}\n\
                                     res[idx] = {}::from_{}_bytes(bytes);\n\
                                     idx += 1;\n\
                                 }}\n\
                                 res\n\
                             }}\n\n",
                        f_name, r_type, len, offset, r_type, len, len, offset, prim_size, prim_size, prim_size, r_type, order_suffix
                    ));

                    src.push_str(&format!(
                        "    pub const fn raw_{}(&self) -> [{}; {}] {{\n\
                                 #[allow(unused_unsafe)]\n\
                                 unsafe {{ self.{}_unchecked() }}\n\
                             }}\n\n",
                        f_name, r_type, len, f_name
                    ));
                } else {
                    if f.presence == Presence::Optional {
                        let null_val = f.null_value.unwrap_or(0);
                        let null_check = if *prim == PrimitiveType::Float {
                            format!("val.to_bits() == {} as u32", null_val)
                        } else if *prim == PrimitiveType::Double {
                            format!("val.to_bits() == {}", null_val)
                        } else {
                            format!("val == {} as {}", null_val, r_type)
                        };

                        src.push_str(&format!(
                            "    pub const fn {}(&self) -> Result<Option<{}>, sbe_rt::DecodeError> {{\n\
                                     let offset = self.pos + {};\n\
                                     if offset + {} > self.buf.len() {{\n\
                                         return Err(sbe_rt::DecodeError::BufferTooShort {{ needed: offset + {}, available: self.buf.len() }});\n\
                                     }}\n\
                                     let mut bytes = [0u8; {}];\n\
                                     let mut j = 0;\n\
                                     while j < {} {{\n\
                                         bytes[j] = self.buf[offset + j];\n\
                                         j += 1;\n\
                                     }}\n\
                                     let val = {}::from_{}_bytes(bytes);\n\
                                     if {} {{\n\
                                         Ok(None)\n\
                                     }} else {{\n\
                                         Ok(Some(val))\n\
                                     }}\n\
                                 }}\n\n",
                            f_name, r_type, offset, prim_size, prim_size, prim_size, prim_size, r_type, order_suffix, null_check
                        ));
                    } else {
                        src.push_str(&format!(
                            "    pub const fn {}(&self) -> Result<{}, sbe_rt::DecodeError> {{\n\
                                     let offset = self.pos + {};\n\
                                     if offset + {} > self.buf.len() {{\n\
                                         return Err(sbe_rt::DecodeError::BufferTooShort {{ needed: offset + {}, available: self.buf.len() }});\n\
                                     }}\n\
                                     let mut bytes = [0u8; {}];\n\
                                     let mut j = 0;\n\
                                     while j < {} {{\n\
                                         bytes[j] = self.buf[offset + j];\n\
                                         j += 1;\n\
                                     }}\n\
                                     Ok({}::from_{}_bytes(bytes))\n\
                                 }}\n\n",
                            f_name, r_type, offset, prim_size, prim_size, prim_size, prim_size, r_type, order_suffix
                        ));
                    }

                    src.push_str(&format!(
                        "    pub const unsafe fn {}_unchecked(&self) -> {} {{\n\
                                 let offset = self.pos + {};\n\
                                 let mut bytes = [0u8; {}];\n\
                                 let mut j = 0;\n\
                                 while j < {} {{\n\
                                     bytes[j] = *self.buf.as_ptr().add(offset + j);\n\
                                     j += 1;\n\
                                 }}\n\
                                 {}::from_{}_bytes(bytes)\n\
                             }}\n\n",
                        f_name, r_type, offset, prim_size, prim_size, r_type, order_suffix
                    ));

                    src.push_str(&format!(
                        "    pub const fn raw_{}(&self) -> {} {{\n\
                                 #[allow(unused_unsafe)]\n\
                                 unsafe {{ self.{}_unchecked() }}\n\
                             }}\n\n",
                        f_name, r_type, f_name
                    ));
                }
            }
            FieldType::Composite { name: comp_name, size: comp_size } => {
                let target_name = to_pascal_case(comp_name);
                src.push_str(&format!(
                    "    pub const fn {}(&self) -> Result<{}, sbe_rt::DecodeError> {{\n\
                             let offset = self.pos + {};\n\
                             if offset + {} > self.buf.len() {{\n\
                                 return Err(sbe_rt::DecodeError::BufferTooShort {{ needed: offset + {}, available: self.buf.len() }});\n\
                             }}\n\
                             let mut bytes = [0u8; {}];\n\
                             let mut j = 0;\n\
                             while j < {} {{\n\
                                 bytes[j] = self.buf[offset + j];\n\
                                 j += 1;\n\
                             }}\n\
                             Ok({}(bytes))\n\
                         }}\n\n",
                    f_name, target_name, offset, comp_size, comp_size, comp_size, comp_size, target_name
                ));
            }
            FieldType::Enum { name: enum_name, encoding_type } => {
                let target_name = to_pascal_case(enum_name);
                let r_type = rust_type(*encoding_type);
                let prim_size = encoding_type.size();

                src.push_str(&format!(
                    "    pub const fn {}(&self) -> Result<{}, sbe_rt::DecodeError> {{\n\
                             let offset = self.pos + {};\n\
                             if offset + {} > self.buf.len() {{\n\
                                 return Err(sbe_rt::DecodeError::BufferTooShort {{ needed: offset + {}, available: self.buf.len() }});\n\
                             }}\n\
                             let mut bytes = [0u8; {}];\n\
                             let mut j = 0;\n\
                             while j < {} {{\n\
                                 bytes[j] = self.buf[offset + j];\n\
                                 j += 1;\n\
                             }}\n\
                             Ok({}({}::from_{}_bytes(bytes)))\n\
                         }}\n\n",
                    f_name, target_name, offset, prim_size, prim_size, prim_size, prim_size, target_name, r_type, order_suffix
                ));
            }
            FieldType::Set { name: set_name, encoding_type } => {
                let target_name = to_pascal_case(set_name);
                let r_type = rust_type(*encoding_type);
                let prim_size = encoding_type.size();

                src.push_str(&format!(
                    "    pub const fn {}(&self) -> Result<{}, sbe_rt::DecodeError> {{\n\
                             let offset = self.pos + {};\n\
                             if offset + {} > self.buf.len() {{\n\
                                 return Err(sbe_rt::DecodeError::BufferTooShort {{ needed: offset + {}, available: self.buf.len() }});\n\
                             }}\n\
                             let mut bytes = [0u8; {}];\n\
                             let mut j = 0;\n\
                             while j < {} {{\n\
                                 bytes[j] = self.buf[offset + j];\n\
                                 j += 1;\n\
                             }}\n\
                             Ok({}({}::from_{}_bytes(bytes)))\n\
                         }}\n\n",
                    f_name, target_name, offset, prim_size, prim_size, prim_size, prim_size, target_name, r_type, order_suffix
                ));
            }
        }
    }

    // Group entry tail offsets
    src.push_str(&format!(
        "    #[inline]\n\
             fn tail_offset_0(&self) -> Result<usize, sbe_rt::DecodeError> {{\n\
                 Ok(self.pos + Self::ENTRY_BLOCK_LENGTH)\n\
             }}\n\n"
    ));

    let mut k = 0;
    for ng in &g.groups {
        let (dim_name, dim_size, bl_field, count_field) = get_dimension_info(elements, &ng.dimension_type);
        let ng_pascal = to_pascal_case(&ng.name);
        src.push_str(&format!(
            "    #[inline]\n\
                 fn tail_offset_{}(&self) -> Result<usize, sbe_rt::DecodeError> {{\n\
                     let start = self.tail_offset_{}()?;\n\
                     if start + {} > self.buf.len() {{\n\
                         return Err(sbe_rt::DecodeError::BufferTooShort {{ needed: start + {}, available: self.buf.len() }});\n\
                     }}\n\
                     let mut bytes = [0u8; {}];\n\
                     let mut j = 0;\n\
                     while j < {} {{\n\
                         bytes[j] = self.buf[start + j];\n\
                         j += 1;\n\
                     }}\n\
                     let header = {}(bytes);\n\
                     let count = header.{}() as usize;\n\
                     let block_len = header.{}() as usize;\n\
                     let mut pos = start + {};\n\
                     let mut idx = 0;\n\
                     while idx < count {{\n\
                         pos = {}EntryDecoder::skip(self.buf, pos, block_len, self.acting_version)?;\n\
                         idx += 1;\n\
                     }}\n\
                     Ok(pos)\n\
                 }}\n\n",
            k + 1, k, dim_size, dim_size, dim_size, dim_size, dim_name, count_field, bl_field, dim_size, ng_pascal
        ));
        k += 1;
    }

    for vd in &g.var_data {
        let (type_pascal, prefix_size, len_field, _) = get_vardata_info(elements, &vd.type_name);
        src.push_str(&format!(
            "    #[inline]\n\
                 fn tail_offset_{}(&self) -> Result<usize, sbe_rt::DecodeError> {{\n\
                     let start = self.tail_offset_{}()?;\n\
                     if start + {} > self.buf.len() {{\n\
                         return Err(sbe_rt::DecodeError::BufferTooShort {{ needed: start + {}, available: self.buf.len() }});\n\
                     }}\n\
                     let mut bytes = [0u8; {}];\n\
                     let mut j = 0;\n\
                     while j < {} {{\n\
                         bytes[j] = self.buf[start + j];\n\
                         j += 1;\n\
                     }}\n\
                     let header = {}(bytes);\n\
                     let len = header.{}() as usize;\n\
                     if start + {} + len > self.buf.len() {{\n\
                         return Err(sbe_rt::DecodeError::BufferTooShort {{ needed: start + {} + len, available: self.buf.len() }});\n\
                     }}\n\
                     Ok(start + {} + len)\n\
                 }}\n\n",
            k + 1, k, prefix_size, prefix_size, prefix_size, prefix_size, type_pascal, len_field, prefix_size, prefix_size, prefix_size
        ));
        k += 1;
    }

    // Accessors for nested groups
    let mut ng_idx = 0;
    for ng in &g.groups {
        let ng_pascal = to_pascal_case(&ng.name);
        let ng_snake = to_snake_case(&ng.name);
        src.push_str(&format!(
            "    pub fn {}(&self) -> Result<{}Decoder<'a>, sbe_rt::DecodeError> {{\n\
                     let offset = self.tail_offset_{}()?;\n\
                     {}Decoder::wrap(self.buf, offset, self.acting_version)\n\
                 }}\n\n",
            ng_snake, ng_pascal, ng_idx, ng_pascal
        ));
        ng_idx += 1;
    }

    // Accessors for nested var_data
    let mut nvd_idx = g.groups.len();
    for vd in &g.var_data {
        let (type_pascal, prefix_size, len_field, _) = get_vardata_info(elements, &vd.type_name);
        let vd_snake = to_snake_case(&vd.name);
        src.push_str(&format!(
            "    pub fn {}(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {{\n\
                     let offset = self.tail_offset_{}()?;\n\
                     let mut bytes = [0u8; {}];\n\
                     let mut j = 0;\n\
                     while j < {} {{\n\
                         bytes[j] = self.buf[offset + j];\n\
                         j += 1;\n\
                     }}\n\
                     let header = {}(bytes);\n\
                     let len = header.{}() as usize;\n\
                     let data_offset = offset + {};\n\
                     Ok(&self.buf[data_offset .. data_offset + len])\n\
                 }}\n\n",
            vd_snake, nvd_idx, prefix_size, prefix_size, type_pascal, len_field, prefix_size
        ));
        nvd_idx += 1;
    }

    src.push_str(&format!(
        "    pub fn encoded_length(&self) -> usize {{\n\
                 self.tail_offset_{}().unwrap_or(self.pos + Self::ENTRY_BLOCK_LENGTH) - self.pos\n\
             }}\n\n",
        total_tail
    ));

    src.push_str(&format!(
        "    pub fn skip(buf: &'a [u8], pos: usize, block_len: usize, acting_version: u16) -> Result<usize, sbe_rt::DecodeError> {{\n\
                 let entry = Self::wrap(buf, pos, acting_version);\n\
                 entry.tail_offset_{}()\n\
             }}\n",
        total_tail
    ));

    src.push_str("}\n\n");

    // Recursively generate nested Repeating Groups decoders
    for ng in &g.groups {
        generate_group_decoder(src, ng, elements, byte_order);
    }
}

fn generate_nullification(src: &mut String, fields: &[MessageField], offset_base: &str, byte_order: ByteOrder) {
    let order_suffix = match byte_order {
        ByteOrder::LittleEndian => "le",
        ByteOrder::BigEndian => "be",
    };
    for f in fields {
        if f.presence == Presence::Optional {
            if let Some(null_val) = f.null_value {
                let size = match f.field_type {
                    FieldType::Primitive(p, length) => p.size() * length.unwrap_or(1),
                    FieldType::Composite { size, .. } => size,
                    FieldType::Enum { encoding_type, .. } => encoding_type.size(),
                    FieldType::Set { encoding_type, .. } => encoding_type.size(),
                };
                src.push_str(&format!(
                    "        let null_bytes = ({}_u64).to_{}_bytes();\n\
                             let offset = {} + {};\n\
                             let mut j = 0;\n\
                             while j < {} {{\n\
                                 buf[offset + j] = null_bytes[j];\n\
                                 j += 1;\n\
                             }}\n",
                    null_val, order_suffix, offset_base, f.offset, size
                ));
            }
        }
    }
}

fn generate_message_encoder(
    src: &mut String,
    msg: &MessageStructure,
    elements: &SchemaElements,
    byte_order: ByteOrder,
    schema_id: u16,
    schema_version: u16,
    header_type: &str,
) {
    let raw_name = &msg.name;
    let name = to_pascal_case(raw_name);
    let order_suffix = match byte_order {
        ByteOrder::LittleEndian => "le",
        ByteOrder::BigEndian => "be",
    };

    let block_length = msg.fields.iter().fold(0, |acc, f| {
        let size = match f.field_type {
            FieldType::Primitive(p, length) => p.size() * length.unwrap_or(1),
            FieldType::Composite { size, .. } => size,
            FieldType::Enum { encoding_type, .. } => encoding_type.size(),
            FieldType::Set { encoding_type, .. } => encoding_type.size(),
        };
        acc.max(f.offset + size)
    });

    let header_pascal = to_pascal_case(header_type);
    let header_size = elements.composites.iter()
        .find(|c| c[0].name == header_type)
        .and_then(|c| c[0].encoding.offset)
        .unwrap_or(8);

    // Generate Encoder phantom states if we have variable length tail elements
    let total_tail = msg.groups.len() + msg.var_data.len();
    if total_tail > 0 {
        src.push_str(&format!("pub mod {}_encoder_state {{\n", to_snake_case(&msg.name)));
        let mut tail_idx = 0;
        for g in &msg.groups {
            src.push_str(&format!("    pub struct Needs{};\n", to_pascal_case(&g.name)));
            tail_idx += 1;
        }
        for vd in &msg.var_data {
            src.push_str(&format!("    pub struct Needs{};\n", to_pascal_case(&vd.name)));
            tail_idx += 1;
        }
        src.push_str("    pub struct Complete;\n");
        src.push_str("}\n\n");
    }

    if total_tail > 0 {
        let first_state = &msg.groups.first().map(|g| to_pascal_case(&g.name))
            .unwrap_or_else(|| to_pascal_case(&msg.var_data.first().unwrap().name));
        src.push_str(&format!(
            "pub struct {}Encoder<'a, State = {}_encoder_state::Needs{}> {{\n\
                 buf: &'a mut [u8],\n\
                 message_start: usize,\n\
                 pos: usize,\n\
                 _phantom: core::marker::PhantomData<State>,\n\
             }}\n\n",
            name, to_snake_case(&msg.name), first_state
        ));
    } else {
        src.push_str(&format!(
            "pub struct {}Encoder<'a> {{\n\
                 buf: &'a mut [u8],\n\
                 message_start: usize,\n\
                 pos: usize,\n\
             }}\n\n",
            name
        ));
    }

    if total_tail > 0 {
        src.push_str(&format!("impl<'a, State> {}Encoder<'a, State> {{\n", name));
    } else {
        src.push_str(&format!("impl<'a> {}Encoder<'a> {{\n", name));
    }

    src.push_str(&format!(
        "    pub const SCHEMA_ID: u16 = {};\n\
             pub const SCHEMA_VERSION: u16 = {};\n\
             pub const TEMPLATE_ID: u16 = {};\n\
             pub const BLOCK_LENGTH: usize = {};\n\n",
        schema_id, schema_version, msg.id, block_length
    ));

    if total_tail > 0 {
        let first_state = &msg.groups.first().map(|g| to_pascal_case(&g.name))
            .unwrap_or_else(|| to_pascal_case(&msg.var_data.first().unwrap().name));
        src.push_str(&format!(
            "    pub fn wrap(buf: &'a mut [u8], pos: usize) -> Self {{\n\
                     Self {{\n\
                         buf,\n\
                         message_start: pos,\n\
                         pos: pos + {} + {},\n\
                         _phantom: core::marker::PhantomData,\n\
                     }}\n\
                 }}\n\n\
                 pub fn wrap_and_apply_header(buf: &'a mut [u8], pos: usize) -> Result<Self, sbe_rt::EncodeError> {{\n\
                     let needed = pos + {} + {};\n\
                     if needed > buf.len() {{\n\
                         return Err(sbe_rt::EncodeError::BufferTooShort {{ needed, available: buf.len() }});\n\
                     }}\n\
                     let header = {}::new(Self::BLOCK_LENGTH as u16, Self::TEMPLATE_ID, Self::SCHEMA_ID, Self::SCHEMA_VERSION);\n\
                     let header_bytes = header.0;\n\
                     let mut j = 0;\n\
                     while j < {} {{\n\
                         buf[pos + j] = header_bytes[j];\n\
                         j += 1;\n\
                     }}\n",
            header_size, block_length, header_size, block_length, header_pascal, header_size
        ));
        generate_nullification(src, &msg.fields, "pos + 8", byte_order);
        src.push_str("        Ok(Self::wrap(buf, pos))\n    }\n\n");
    } else {
        src.push_str(&format!(
            "    pub fn wrap(buf: &'a mut [u8], pos: usize) -> Self {{\n\
                     Self {{\n\
                         buf,\n\
                         message_start: pos,\n\
                         pos: pos + {} + {},\n\
                     }}\n\
                 }}\n\n\
                 pub fn wrap_and_apply_header(buf: &'a mut [u8], pos: usize) -> Result<Self, sbe_rt::EncodeError> {{\n\
                     let needed = pos + {} + {};\n\
                     if needed > buf.len() {{\n\
                         return Err(sbe_rt::EncodeError::BufferTooShort {{ needed, available: buf.len() }});\n\
                     }}\n\
                     let header = {}::new(Self::BLOCK_LENGTH as u16, Self::TEMPLATE_ID, Self::SCHEMA_ID, Self::SCHEMA_VERSION);\n\
                     let header_bytes = header.0;\n\
                     let mut j = 0;\n\
                     while j < {} {{\n\
                         buf[pos + j] = header_bytes[j];\n\
                         j += 1;\n\
                     }}\n",
            header_size, block_length, header_size, block_length, header_pascal, header_size
        ));
        generate_nullification(src, &msg.fields, "pos + 8", byte_order);
        src.push_str("        Ok(Self::wrap(buf, pos))\n    }\n\n");
    }

    // Setters for fixed fields
    for f in &msg.fields {
        let f_name = to_snake_case(&f.name);
        let offset = f.offset;
        let since = f.since_version;

        match &f.field_type {
            FieldType::Primitive(prim, length) => {
                let r_type = rust_type(*prim);
                let prim_size = prim.size();
                if f.presence == Presence::Constant {
                    // Constant fields have no setter
                } else if let Some(len) = length {
                    src.push_str(&format!(
                        "    pub fn {}(&mut self, val: [{}; {}]) -> &mut Self {{\n\
                                 let offset = self.message_start + {} + {};\n\
                                 let mut idx = 0;\n\
                                 while idx < {} {{\n\
                                     let val_bytes = val[idx].to_{}_bytes();\n\
                                     let mut j = 0;\n\
                                     while j < {} {{\n\
                                         self.buf[offset + idx * {} + j] = val_bytes[j];\n\
                                         j += 1;\n\
                                     }}\n\
                                     idx += 1;\n\
                                 }}\n\
                                 self\n\
                             }}\n\n",
                        f_name, r_type, len, header_size, offset, len, order_suffix, prim_size, prim_size
                    ));
                } else {
                    src.push_str(&format!(
                        "    pub fn {}(&mut self, val: {}) -> &mut Self {{\n\
                                 let offset = self.message_start + {} + {};\n\
                                 let val_bytes = val.to_{}_bytes();\n\
                                 let mut j = 0;\n\
                                 while j < {} {{\n\
                                     self.buf[offset + j] = val_bytes[j];\n\
                                     j += 1;\n\
                                 }}\n\
                                 self\n\
                             }}\n\n",
                        f_name, r_type, header_size, offset, order_suffix, prim_size
                    ));
                }
            }
            FieldType::Composite { name: comp_name, size: comp_size } => {
                let target_name = to_pascal_case(comp_name);
                src.push_str(&format!(
                    "    pub fn {}(&mut self, val: {}) -> &mut Self {{\n\
                             let offset = self.message_start + {} + {};\n\
                             let mut j = 0;\n\
                             while j < {} {{\n\
                                 self.buf[offset + j] = val.0[j];\n\
                                 j += 1;\n\
                             }}\n\
                             self\n\
                         }}\n\n",
                    f_name, target_name, header_size, offset, comp_size
                ));
            }
            FieldType::Enum { name: enum_name, encoding_type } => {
                let target_name = to_pascal_case(enum_name);
                let prim_size = encoding_type.size();
                src.push_str(&format!(
                    "    pub fn {}(&mut self, val: {}) -> &mut Self {{\n\
                             let offset = self.message_start + {} + {};\n\
                             let val_bytes = val.0.to_{}_bytes();\n\
                             let mut j = 0;\n\
                             while j < {} {{\n\
                                 self.buf[offset + j] = val_bytes[j];\n\
                                 j += 1;\n\
                             }}\n\
                             self\n\
                         }}\n\n",
                    f_name, target_name, header_size, offset, order_suffix, prim_size
                ));
            }
            FieldType::Set { name: set_name, encoding_type } => {
                let target_name = to_pascal_case(set_name);
                let prim_size = encoding_type.size();
                src.push_str(&format!(
                    "    pub fn {}(&mut self, val: {}) -> &mut Self {{\n\
                             let offset = self.message_start + {} + {};\n\
                             let val_bytes = val.0.to_{}_bytes();\n\
                             let mut j = 0;\n\
                             while j < {} {{\n\
                                 self.buf[offset + j] = val_bytes[j];\n\
                                 j += 1;\n\
                             }}\n\
                             self\n\
                         }}\n\n",
                    f_name, target_name, header_size, offset, order_suffix, prim_size
                ));
            }
        }
    }

    src.push_str(&format!(
        "    pub fn encoded_length(&self) -> usize {{\n\
                 self.pos - (self.message_start + {})\n\
             }}\n\n\
             pub fn encoded_length_with_header(&self) -> usize {{\n\
                 self.pos - self.message_start\n\
             }}\n",
        header_size
    ));
    src.push_str("}\n\n");

    // Tail state transition methods
    if total_tail > 0 {
        let mut tail_idx = 0;
        // Group methods
        for g in &msg.groups {
            let next_state = if tail_idx + 1 < total_tail {
                let next_name = if tail_idx + 1 < msg.groups.len() {
                    to_pascal_case(&msg.groups[tail_idx + 1].name)
                } else {
                    to_pascal_case(&msg.var_data[tail_idx + 1 - msg.groups.len()].name)
                };
                format!("{}_encoder_state::Needs{}", to_snake_case(&msg.name), next_name)
            } else {
                format!("{}_encoder_state::Complete", to_snake_case(&msg.name))
            };

            let g_pascal = to_pascal_case(&g.name);
            let g_snake = to_snake_case(&g.name);
            let (dim_name, dim_size, _, _) = get_dimension_info(elements, &g.dimension_type);

            src.push_str(&format!(
                "impl<'a> {}Encoder<'a, {}_encoder_state::Needs{}> {{\n\
                     pub fn {}<F>(mut self, count: u16, f: F) -> Result<{}Encoder<'a, {}>, sbe_rt::EncodeError>\n\
                     where\n\
                         F: FnOnce(&mut {}Encoder<'a>),\n\
                     {{\n\
                         if self.pos + {} > self.buf.len() {{\n\
                             return Err(sbe_rt::EncodeError::BufferTooShort {{ needed: self.pos + {}, available: self.buf.len() }});\n\
                         }}\n\
                         let header = {};\n\
                         let header_bytes = header.0;\n\
                         let mut j = 0;\n\
                         while j < {} {{\n\
                             self.buf[self.pos + j] = header_bytes[j];\n\
                             j += 1;\n\
                         }}\n\
                         let mut group = {}Encoder::wrap(self.buf, self.pos + {}, count);\n\
                         f(&mut group);\n\
                         Ok({}Encoder {{\n\
                             buf: self.buf,\n\
                             message_start: self.message_start,\n\
                             pos: group.pos,\n\
                             _phantom: core::marker::PhantomData,\n\
                         }})\n\
                     }}\n\n\
                 }}\n\n",
                name, to_snake_case(&msg.name), g_pascal, g_snake, name, next_state, g_pascal, dim_size, dim_size,
                generate_dim_new_call(elements, &g.dimension_type, &format!("{}Encoder::ENTRY_BLOCK_LENGTH as u16", g_pascal), "count"),
                dim_size, g_pascal, dim_size, name
            ));
            tail_idx += 1;
        }

        // VarData methods
        for vd in &msg.var_data {
            let next_state = if tail_idx + 1 < total_tail {
                let next_name = to_pascal_case(&msg.var_data[tail_idx + 1 - msg.groups.len()].name);
                format!("{}_encoder_state::Needs{}", to_snake_case(&msg.name), next_name)
            } else {
                format!("{}_encoder_state::Complete", to_snake_case(&msg.name))
            };

            let vd_pascal = to_pascal_case(&vd.name);
            let vd_snake = to_snake_case(&vd.name);
            let (_, prefix_size, _, len_type) = get_vardata_info(elements, &vd.type_name);
            let len_rust_type = rust_type(len_type);

            src.push_str(&format!(
                "impl<'a> {}Encoder<'a, {}_encoder_state::Needs{}> {{\n\
                     pub fn {}(mut self, data: &[u8]) -> Result<{}Encoder<'a, {}>, sbe_rt::EncodeError> {{\n\
                         let needed = self.pos + {} + data.len();\n\
                         if needed > self.buf.len() {{\n\
                             return Err(sbe_rt::EncodeError::BufferTooShort {{ needed, available: self.buf.len() }});\n\
                         }}\n\
                         let len_bytes = (data.len() as {}).to_{}_bytes();\n\
                         let mut j = 0;\n\
                         while j < {} {{\n\
                             self.buf[self.pos + j] = len_bytes[j];\n\
                             j += 1;\n\
                         }}\n\
                         let start = self.pos + {};\n\
                         let mut d = 0;\n\
                         while d < data.len() {{\n\
                             self.buf[start + d] = data[d];\n\
                             d += 1;\n\
                         }}\n\
                         Ok({}Encoder {{\n\
                             buf: self.buf,\n\
                             message_start: self.message_start,\n\
                             pos: start + data.len(),\n\
                             _phantom: core::marker::PhantomData,\n\
                         }})\n\
                     }}\n\
                 }}\n\n",
                name, to_snake_case(&msg.name), vd_pascal, vd_snake, name, next_state, prefix_size, len_rust_type, order_suffix, prefix_size, prefix_size, name
            ));
            tail_idx += 1;
        }

        // Complete state impl
        src.push_str(&format!(
            "impl<'a> {}Encoder<'a, {}_encoder_state::Complete> {{\n\
                 pub fn as_bytes(&self) -> &[u8] {{\n\
                     &self.buf[self.message_start .. self.pos]\n\
                 }}\n\
             }}\n\n\
             impl<'a> AsRef<[u8]> for {}Encoder<'a, {}_encoder_state::Complete> {{\n\
                 fn as_ref(&self) -> &[u8] {{\n\
                     self.as_bytes()\n\
                 }}\n\
             }}\n\n",
            name, to_snake_case(&msg.name), name, to_snake_case(&msg.name)
        ));
    } else {
        src.push_str(&format!(
            "impl<'a> AsRef<[u8]> for {}Encoder<'a> {{\n\
                 fn as_ref(&self) -> &[u8] {{\n\
                     &self.buf[self.message_start .. self.pos]\n\
                 }}\n\
             }}\n\n",
            name
        ));
    }

    if total_tail > 0 {
        src.push_str(&format!(
            "impl<'a, State> sbe_rt::private::Sealed for {}Encoder<'a, State> {{}}\n\n\
             impl<'a, State> sbe_rt::SbeMessage for {}Encoder<'a, State> {{\n\
                 const TEMPLATE_ID: u16 = {};\n\
                 const BLOCK_LENGTH: usize = {};\n\
                 const SCHEMA_ID: u16 = {};\n\
                 const SCHEMA_VERSION: u16 = {};\n\
             }}\n\n",
            name, name, msg.id, block_length, schema_id, schema_version
        ));
    } else {
        src.push_str(&format!(
            "impl<'a> sbe_rt::private::Sealed for {}Encoder<'a> {{}}\n\n\
             impl<'a> sbe_rt::SbeMessage for {}Encoder<'a> {{\n\
                 const TEMPLATE_ID: u16 = {};\n\
                 const BLOCK_LENGTH: usize = {};\n\
                 const SCHEMA_ID: u16 = {};\n\
                 const SCHEMA_VERSION: u16 = {};\n\
             }}\n\n",
            name, name, msg.id, block_length, schema_id, schema_version
        ));
    }

    // Recursively generate Repeating Groups encoders for this message
    for g in &msg.groups {
        generate_group_encoder(src, g, elements, byte_order);
    }
}

fn generate_group_encoder(src: &mut String, g: &MessageGroup, elements: &SchemaElements, byte_order: ByteOrder) {
    let name = to_pascal_case(&g.name);
    let order_suffix = match byte_order {
        ByteOrder::LittleEndian => "le",
        ByteOrder::BigEndian => "be",
    };

    src.push_str(&format!(
        "pub struct {}Encoder<'a> {{\n\
             buf: &'a mut [u8],\n\
             pos: usize,\n\
             count: u16,\n\
             written: u16,\n\
         }}\n\n\
         impl<'a> {}Encoder<'a> {{\n\
             pub const ENTRY_BLOCK_LENGTH: usize = {};\n\n\
             pub fn wrap(buf: &'a mut [u8], pos: usize, count: u16) -> Self {{\n\
                 Self {{ buf, pos, count, written: 0 }}\n\
             }}\n\n\
             pub fn add<F>(&mut self, f: F) -> Result<(), sbe_rt::EncodeError>\n\
             where\n\
                 F: FnOnce(&mut {}EntryEncoder<'a>),\n\
             {{\n\
                 if self.written >= self.count {{\n\
                     return Ok(());\n\
                 }}\n\
                 let block_len = Self::ENTRY_BLOCK_LENGTH;\n\
                 if self.pos + block_len > self.buf.len() {{\n\
                     return Err(sbe_rt::EncodeError::BufferTooShort {{ needed: self.pos + block_len, available: self.buf.len() }});\n\
                 }}\n\
                 let mut entry = {}EntryEncoder::wrap(self.buf, self.pos);\n",
        name, name, g.block_length, name, name
    ));

    generate_nullification(src, &g.fields, "self.pos", byte_order);

    src.push_str(&format!(
        "        f(&mut entry);\n\
                 self.pos = entry.pos;\n\
                 self.written += 1;\n\
                 Ok(())\n\
             }}\n\
         }}\n\n"
    ));

    // Entry Encoder Struct
    src.push_str(&format!(
        "pub struct {}EntryEncoder<'a> {{\n\
             buf: &'a mut [u8],\n\
             entry_start: usize,\n\
             pos: usize,\n\
         }}\n\n\
         impl<'a> {}EntryEncoder<'a> {{\n\
             pub const ENTRY_BLOCK_LENGTH: usize = {};\n\n\
             pub fn wrap(buf: &'a mut [u8], pos: usize) -> Self {{\n\
                 Self {{\n\
                     buf,\n\
                     entry_start: pos,\n\
                     pos: pos + Self::ENTRY_BLOCK_LENGTH,\n\
                 }}\n\
             }}\n\n",
        name, name, g.block_length
    ));

    // Setters for group entry fields
    for f in &g.fields {
        let f_name = to_snake_case(&f.name);
        let offset = f.offset;
        let since = f.since_version;

        match &f.field_type {
            FieldType::Primitive(prim, length) => {
                let r_type = rust_type(*prim);
                let prim_size = prim.size();
                if f.presence == Presence::Constant {
                    // Constant fields have no setter
                } else if let Some(len) = length {
                    src.push_str(&format!(
                        "    pub fn {}(&mut self, val: [{}; {}]) -> &mut Self {{\n\
                                 let offset = self.entry_start + {};\n\
                                 let mut idx = 0;\n\
                                 while idx < {} {{\n\
                                     let val_bytes = val[idx].to_{}_bytes();\n\
                                     let mut j = 0;\n\
                                     while j < {} {{\n\
                                         self.buf[offset + idx * {} + j] = val_bytes[j];\n\
                                         j += 1;\n\
                                     }}\n\
                                     idx += 1;\n\
                                 }}\n\
                                 self\n\
                             }}\n\n",
                        f_name, r_type, len, offset, len, order_suffix, prim_size, prim_size
                    ));
                } else {
                    src.push_str(&format!(
                        "    pub fn {}(&mut self, val: {}) -> &mut Self {{\n\
                                 let offset = self.entry_start + {};\n\
                                 let val_bytes = val.to_{}_bytes();\n\
                                 let mut j = 0;\n\
                                 while j < {} {{\n\
                                     self.buf[offset + j] = val_bytes[j];\n\
                                     j += 1;\n\
                                 }}\n\
                                 self\n\
                             }}\n\n",
                        f_name, r_type, offset, order_suffix, prim_size
                    ));
                }
            }
            FieldType::Composite { name: comp_name, size: comp_size } => {
                let target_name = to_pascal_case(comp_name);
                src.push_str(&format!(
                    "    pub fn {}(&mut self, val: {}) -> &mut Self {{\n\
                             let offset = self.entry_start + {};\n\
                             let mut j = 0;\n\
                             while j < {} {{\n\
                                 self.buf[offset + j] = val.0[j];\n\
                                 j += 1;\n\
                             }}\n\
                             self\n\
                         }}\n\n",
                    f_name, target_name, offset, comp_size
                ));
            }
            FieldType::Enum { name: enum_name, encoding_type } => {
                let target_name = to_pascal_case(enum_name);
                let prim_size = encoding_type.size();
                src.push_str(&format!(
                    "    pub fn {}(&mut self, val: {}) -> &mut Self {{\n\
                             let offset = self.entry_start + {};\n\
                             let val_bytes = val.0.to_{}_bytes();\n\
                             let mut j = 0;\n\
                             while j < {} {{\n\
                                 self.buf[offset + j] = val_bytes[j];\n\
                                 j += 1;\n\
                             }}\n\
                             self\n\
                         }}\n\n",
                    f_name, target_name, offset, order_suffix, prim_size
                ));
            }
            FieldType::Set { name: set_name, encoding_type } => {
                let target_name = to_pascal_case(set_name);
                let prim_size = encoding_type.size();
                src.push_str(&format!(
                    "    pub fn {}(&mut self, val: {}) -> &mut Self {{\n\
                             let offset = self.entry_start + {};\n\
                             let val_bytes = val.0.to_{}_bytes();\n\
                             let mut j = 0;\n\
                             while j < {} {{\n\
                                 self.buf[offset + j] = val_bytes[j];\n\
                                 j += 1;\n\
                             }}\n\
                             self\n\
                         }}\n\n",
                    f_name, target_name, offset, order_suffix, prim_size
                ));
            }
        }
    }

    // Setters for nested groups in entry
    let mut tail_idx = 0;
    let total_tail = g.groups.len() + g.var_data.len();
    for ng in &g.groups {
        let ng_pascal = to_pascal_case(&ng.name);
        let ng_snake = to_snake_case(&ng.name);
        let (dim_name, dim_size, _, _) = get_dimension_info(elements, &ng.dimension_type);
        src.push_str(&format!(
            "    pub fn {}<F>(&mut self, count: u16, f: F) -> Result<&mut Self, sbe_rt::EncodeError>\n\
                 where\n\
                     F: FnOnce(&mut {}Encoder<'a>),\n\
                 {{\n\
                     if self.pos + {} > self.buf.len() {{\n\
                         return Err(sbe_rt::EncodeError::BufferTooShort {{ needed: self.pos + {}, available: self.buf.len() }});\n\
                     }}\n\
                     let header = {};\n\
                     let header_bytes = header.0;\n\
                     let mut j = 0;\n\
                     while j < {} {{\n\
                         self.buf[self.pos + j] = header_bytes[j];\n\
                         j += 1;\n\
                     }}\n\
                     let mut group = {}Encoder::wrap(self.buf, self.pos + {}, count);\n\
                     f(&mut group);\n\
                     self.pos = group.pos;\n\
                     Ok(self)\n\
                 }}\n\n",
            ng_snake, ng_pascal, dim_size, dim_size,
            generate_dim_new_call(elements, &ng.dimension_type, &format!("{}Encoder::ENTRY_BLOCK_LENGTH as u16", ng_pascal), "count"),
            dim_size, ng_pascal, dim_size
        ));
        tail_idx += 1;
    }

    // Setters for nested var_data in entry
    for vd in &g.var_data {
        let vd_snake = to_snake_case(&vd.name);
        let (_, prefix_size, _, len_type) = get_vardata_info(elements, &vd.type_name);
        let len_rust_type = rust_type(len_type);
        src.push_str(&format!(
            "    pub fn {}(&mut self, data: &[u8]) -> Result<&mut Self, sbe_rt::EncodeError> {{\n\
                     let needed = self.pos + {} + data.len();\n\
                     if needed > self.buf.len() {{\n\
                         return Err(sbe_rt::EncodeError::BufferTooShort {{ needed, available: self.buf.len() }});\n\
                     }}\n\
                     let len_bytes = (data.len() as {}).to_{}_bytes();\n\
                     let mut j = 0;\n\
                     while j < {} {{\n\
                         self.buf[self.pos + j] = len_bytes[j];\n\
                         j += 1;\n\
                     }}\n\
                     let start = self.pos + {};\n\
                     let mut d = 0;\n\
                     while d < data.len() {{\n\
                         self.buf[start + d] = data[d];\n\
                         d += 1;\n\
                     }}\n\
                     self.pos = start + data.len();\n\
                     Ok(self)\n\
                 }}\n\n",
            vd_snake, prefix_size, len_rust_type, order_suffix, prefix_size, prefix_size
        ));
        tail_idx += 1;
    }

    src.push_str("}\n\n");

    // Recursively generate nested Repeating Groups encoders
    for ng in &g.groups {
        generate_group_encoder(src, ng, elements, byte_order);
    }
}

fn generate_any_message(
    src: &mut String,
    messages: &[MessageStructure],
    elements: &SchemaElements,
    schema_id: u16,
    header_type: &str,
) {
    let header_size = elements.composites.iter()
        .find(|c| c[0].name == header_type)
        .and_then(|c| c[0].encoding.offset)
        .unwrap_or(8);

    let (header_bl, header_ti, header_si, header_vr) = {
        let mut bl = "block_length".to_string();
        let mut ti = "template_id".to_string();
        let mut si = "schema_id".to_string();
        let mut vr = "version".to_string();
        if let Some(comp) = elements.composites.iter().find(|c| c[0].name == header_type) {
            let members = parse_composite_members(comp);
            for m in members {
                let lower = m.name.to_lowercase();
                if lower.contains("blocklength") {
                    bl = to_snake_case(&m.name);
                } else if lower.contains("templateid") {
                    ti = to_snake_case(&m.name);
                } else if lower.contains("schemaid") {
                    si = to_snake_case(&m.name);
                } else if lower.contains("version") {
                    vr = to_snake_case(&m.name);
                }
            }
        }
        (bl, ti, si, vr)
    };

    src.push_str(
        "#[non_exhaustive]\n\
         #[derive(Clone, Copy)]\n\
         pub enum AnyMessage<'a> {\n"
    );
    for m in messages {
        let name_pascal = to_pascal_case(&m.name);
        src.push_str(&format!("    {}({}Decoder<'a>),\n", name_pascal, name_pascal));
    }
    src.push_str(&format!(
        "    Unknown {{\n\
                 header: {},\n\
                 payload: &'a [u8],\n\
             }},\n\
         }}\n\n",
        to_pascal_case(header_type)
    ));

    // DecodedFrame Struct
    src.push_str(
        "#[derive(Clone)]\n\
         pub struct DecodedFrame<'a> {\n\
             pub message: AnyMessage<'a>,\n\
             pub range: core::ops::Range<usize>,\n\
             pub len: usize,\n\
         }\n\n"
    );

    // FramingPolicy Enum
    src.push_str(
        "#[derive(Clone, Copy, Debug, PartialEq, Eq)]\n\
         pub enum FramingPolicy {\n\
             LengthPrefixU32,\n\
             LengthPrefixU16,\n\
             Fixed(usize),\n\
         }\n\n"
    );

    // FrameCursor Struct
    src.push_str(
        "pub struct FrameCursor<'a> {\n\
             buf: &'a [u8],\n\
             pos: usize,\n\
             framing: FramingPolicy,\n\
         }\n\n\
         impl<'a> FrameCursor<'a> {\n\
             pub const fn new(buf: &'a [u8], framing: FramingPolicy) -> Self {\n\
                 Self { buf, pos: 0, framing }\n\
             }\n\
         }\n\n\
         impl<'a> Iterator for FrameCursor<'a> {\n\
             type Item = Result<DecodedFrame<'a>, sbe_rt::DecodeError>;\n\
             fn next(&mut self) -> Option<Self::Item> {\n\
                 if self.pos >= self.buf.len() {\n\
                     return None;\n\
                 }\n\
                 let (header_len, frame_len) = match self.framing {\n\
                     FramingPolicy::LengthPrefixU32 => {\n\
                         if self.pos + 4 > self.buf.len() {\n\
                             return Some(Err(sbe_rt::DecodeError::BufferTooShort { needed: self.pos + 4, available: self.buf.len() }));\n\
                         }\n\
                         let mut bytes = [0u8; 4];\n\
                         let mut j = 0;\n\
                         while j < 4 {\n\
                             bytes[j] = self.buf[self.pos + j];\n\
                             j += 1;\n\
                         }\n\
                         let len = u32::from_le_bytes(bytes) as usize;\n\
                         (4, len)\n\
                     }\n\
                     FramingPolicy::LengthPrefixU16 => {\n\
                         if self.pos + 2 > self.buf.len() {\n\
                             return Some(Err(sbe_rt::DecodeError::BufferTooShort { needed: self.pos + 2, available: self.buf.len() }));\n\
                         }\n\
                         let mut bytes = [0u8; 2];\n\
                         let mut j = 0;\n\
                         while j < 2 {\n\
                             bytes[j] = self.buf[self.pos + j];\n\
                             j += 1;\n\
                         }\n\
                         let len = u16::from_le_bytes(bytes) as usize;\n\
                         (2, len)\n\
                     }\n\
                     FramingPolicy::Fixed(len) => (0, len),\n\
                 };\n\n\
                 if self.pos + header_len + frame_len > self.buf.len() {\n\
                     return Some(Err(sbe_rt::DecodeError::BufferTooShort { needed: self.pos + header_len + frame_len, available: self.buf.len() }));\n\
                 }\n\
                 let off = self.pos + header_len;\n\
                 let res = AnyMessage::decode_frame(self.buf, off, frame_len);\n\
                 match res {\n\
                     Ok(frame) => {\n\
                         self.pos += header_len + frame_len;\n\
                         Some(Ok(frame))\n\
                     }\n\
                     Err(e) => Some(Err(e)),\n\
                 }\n\
             }\n\
         }\n\n"
    );

    src.push_str(&format!(
        "impl<'a> AnyMessage<'a> {{\n\
             pub const fn decode(buf: &'a [u8], pos: usize) -> Result<Self, sbe_rt::DecodeError> {{\n\
                 if pos + {} > buf.len() {{\n\
                     return Err(sbe_rt::DecodeError::BufferTooShort {{ needed: pos + {}, available: buf.len() }});\n\
                 }}\n\
                 let mut header_bytes = [0u8; {}];\n\
                 let mut j = 0;\n\
                 while j < {} {{\n\
                     header_bytes[j] = buf[pos + j];\n\
                     j += 1;\n\
                 }}\n\
                 let header = {}(header_bytes);\n\
                 let template_id = header.{}();\n\
                 let schema_id = header.{}();\n\
                 let version = header.{}();\n\
                 let block_length = header.{}() as usize;\n\
                 let body_pos = pos + {};\n\n\
                 if schema_id != {} {{\n\
                     return Err(sbe_rt::DecodeError::WrongSchema {{ expected: {}, actual: schema_id }});\n\
                 }}\n\n\
                 match template_id {{\n",
        header_size, header_size, header_size, header_size, to_pascal_case(header_type), header_ti, header_si, header_vr, header_bl, header_size, schema_id, schema_id
    ));

    for m in messages {
        let name_pascal = to_pascal_case(&m.name);
        src.push_str(&format!(
            "            {} => Ok(Self::{}({}Decoder::wrap(buf, body_pos, block_length, version))),\n",
            m.id, name_pascal, name_pascal
        ));
    }

    src.push_str(
        "            _ => Err(sbe_rt::DecodeError::UnknownTemplateLength { template_id }),\n\
                 }\n\
             }\n\n"
    );

    src.push_str(&format!(
        "    pub const fn decode_frame(buf: &'a [u8], pos: usize, frame_len: usize) -> Result<DecodedFrame<'a>, sbe_rt::DecodeError> {{\n\
                 if pos + {} > buf.len() {{\n\
                     return Err(sbe_rt::DecodeError::BufferTooShort {{ needed: pos + {}, available: buf.len() }});\n\
                 }}\n\
                 let mut header_bytes = [0u8; {}];\n\
                 let mut j = 0;\n\
                 while j < {} {{\n\
                     header_bytes[j] = buf[pos + j];\n\
                     j += 1;\n\
                 }}\n\
                 let header = {}(header_bytes);\n\
                 let template_id = header.{}();\n\
                 let schema_id = header.{}();\n\
                 let version = header.{}();\n\
                 let block_length = header.{}() as usize;\n\
                 let body_pos = pos + {};\n\n\
                 if schema_id != {} {{\n\
                     return Err(sbe_rt::DecodeError::WrongSchema {{ expected: {}, actual: schema_id }});\n\
                 }}\n\n\
                 match template_id {{\n",
        header_size, header_size, header_size, header_size, to_pascal_case(header_type), header_ti, header_si, header_vr, header_bl, header_size, schema_id, schema_id
    ));

    for m in messages {
        let name_pascal = to_pascal_case(&m.name);
        src.push_str(&format!(
            "            {} => {{\n\
                             let decoder = {}Decoder::wrap(buf, body_pos, block_length, version);\n\
                             let total_len = match decoder.encoded_length_with_header() {{\n\
                                 Ok(len) => len,\n\
                                 Err(e) => return Err(e),\n\
                             }};\n\
                             if total_len > frame_len {{\n\
                                 return Err(sbe_rt::DecodeError::BufferTooShort {{ needed: total_len, available: frame_len }});\n\
                             }}\n\
                             Ok(DecodedFrame {{\n\
                                 message: Self::{}(decoder),\n\
                                 range: pos .. pos + total_len,\n\
                                 len: total_len,\n\
                             }})\n\
                         }}\n",
            m.id, name_pascal, name_pascal
        ));
    }

    src.push_str(
        "            _ => {\n\
                             if pos + frame_len > buf.len() {\n\
                                 return Err(sbe_rt::DecodeError::BufferTooShort { needed: pos + frame_len, available: buf.len() });\n\
                             }\n\
                             let payload = &buf[body_pos .. pos + frame_len];\n\
                             Ok(DecodedFrame {\n\
                                 message: Self::Unknown {\n\
                                     header,\n\
                                     payload,\n\
                                 },\n\
                                 range: pos .. pos + frame_len,\n\
                                 len: frame_len,\n\
                             })\n\
                         }\n\
                 }\n\
             }\n"
    );

    src.push_str("}\n\n");
}

#[cfg(test)]
mod tests {
    use crate::{GenerationConfig, Schema};
    use super::Generator;

    #[test]
    fn generator_emits_deterministic_module_name() {
        let generator = Generator::new(GenerationConfig::low_latency("market_data"));
        let schema = Schema::new("fix.sbe", 1, 0);

        let modules = generator.generate(&schema);
        let collected = modules.modules().collect::<Vec<_>>();

        assert_eq!(collected.len(), 1);
        assert_eq!(collected[0].path, "market_data.rs");
        assert!(collected[0].source.contains("fix.sbe"));
    }
}
