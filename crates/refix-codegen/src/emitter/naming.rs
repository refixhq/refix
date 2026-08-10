use refix_dictionary::{DataType, EnumValue, Field};

use crate::emitter::Error;
use crate::{pascal_case, snake_case};

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

pub(super) fn variant_name(field: &Field, value: &EnumValue) -> Result<String, Error> {
    let name = pascal_case(&value.description);
    let starts_with_letter = name.chars().next().is_some_and(|c| c.is_ascii_alphabetic());
    let alphanumeric = name.chars().all(|c| c.is_ascii_alphanumeric());
    if !starts_with_letter || !alphanumeric || name == "Self" {
        return Err(Error::UnrepresentableValue {
            field: field.name.clone(),
            description: value.description.clone(),
        });
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
