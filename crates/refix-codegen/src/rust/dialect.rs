use refix_dictionary::Dictionary;

use super::layout::{MAX_WIDTH, slice};
use crate::dialect::length_tags;

/// The `DIALECT` constant, with every length tag the dictionary links.
pub(super) fn emit_dialect(dictionary: &Dictionary) -> String {
    let tags: Vec<String> = length_tags(dictionary)
        .iter()
        .map(|tag| format!("Tag({tag})"))
        .collect();
    if tags.is_empty() {
        return "pub const DIALECT: Dialect<'static> = Dialect::new();\n".to_owned();
    }
    slice(
        "pub const DIALECT: Dialect<'static> = Dialect::new().with_extra_length_tags(",
        &tags,
        ");",
        MAX_WIDTH,
    )
}
