use refix_dictionary::{DataType, Field};

use crate::emitter::Error;
use crate::snake_case;

pub(super) fn method_name(field: &Field) -> Result<String, Error> {
    let mut name = snake_case(&field.name);
    if let DataType::Other(_) = field.data_type {
        name.push_str("_raw");
    }
    if matches!(name.as_str(), "self" | "super" | "crate") {
        return Err(Error::UnrepresentableName {
            field: field.name.clone(),
        });
    }
    if RESERVED_WORDS.contains(&name.as_str()) {
        return Ok(format!("r#{name}"));
    }
    Ok(name)
}

/// Rust's strict and reserved keywords.
const RESERVED_WORDS: &[&str] = &[
    "abstract", "as", "async", "await", "become", "box", "break", "const", "continue", "do", "dyn",
    "else", "enum", "extern", "false", "final", "fn", "for", "gen", "if", "impl", "in", "let",
    "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref", "return",
    "static", "struct", "trait", "true", "try", "type", "typeof", "unsafe", "unsized", "use",
    "virtual", "where", "while", "yield",
];
