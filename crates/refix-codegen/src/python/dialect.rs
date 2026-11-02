use refix_dictionary::Dictionary;

use super::layout::{MAX_WIDTH, tuple_lines};
use crate::dialect::length_tags;

/// The `DIALECT` constant, with every length tag the dictionary links.
pub(super) fn emit_dialect(dictionary: &Dictionary) -> String {
    let tags: Vec<String> = length_tags(dictionary)
        .iter()
        .map(ToString::to_string)
        .collect();
    if tags.is_empty() {
        return "DIALECT = Dialect()\n".to_owned();
    }
    tuple_lines(
        "DIALECT = Dialect(extra_length_tags=",
        &tags,
        ")",
        MAX_WIDTH,
    )
}
