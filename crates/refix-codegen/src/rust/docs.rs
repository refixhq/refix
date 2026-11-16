/// `text` with everything outside printable ASCII escaped, so it stays on
/// the comment's line.
pub(super) fn plain(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        if c == ' ' || c.is_ascii_graphic() {
            escaped.push(c);
        } else {
            escaped.extend(c.escape_default());
        }
    }
    escaped
}

/// `text` as inline code, fenced by more backticks than it holds in a row.
pub(super) fn code(text: &str) -> String {
    let text = plain(text);
    let longest_run = text.split(|c| c != '`').map(str::len).max().unwrap_or(0);
    if longest_run == 0 {
        return format!("`{text}`");
    }
    let fence = "`".repeat(longest_run + 1);
    format!("{fence} {text} {fence}")
}

/// The attribute that lets a search for `fix_name` find `item`.
///
/// There is none when the name is the item's own, which rustc rejects, or
/// when it is not a plain word.
fn alias(fix_name: &str, item: &str) -> Option<String> {
    let is_word = !fix_name.is_empty()
        && fix_name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_');
    let item = item.strip_prefix("r#").unwrap_or(item);
    (is_word && fix_name != item).then(|| format!("#[doc(alias = \"{fix_name}\")]"))
}

/// A member's doc comment of `summary` and, where there is one, the alias
/// for `fix_name`.
pub(super) fn member_docs(summary: &str, fix_name: &str, item: &str) -> String {
    let mut docs = format!("    /// {summary}\n");
    if let Some(alias) = alias(fix_name, item) {
        docs.push_str(&format!("    {alias}\n"));
    }
    docs
}

#[cfg(test)]
mod tests {
    use super::{alias, code, member_docs, plain};

    #[test]
    fn member_docs_hold_the_summary_and_the_alias() {
        assert_eq!(
            member_docs("The `ClOrdID` (11) field.", "ClOrdID", "cl_ord_id"),
            "    /// The `ClOrdID` (11) field.\n    #[doc(alias = \"ClOrdID\")]\n"
        );
        assert_eq!(
            member_docs("The `symbol` (55) field.", "symbol", "symbol"),
            "    /// The `symbol` (55) field.\n"
        );
    }

    #[test]
    fn plain_text_escapes_what_would_end_the_line() {
        assert_eq!(plain("35=D"), "35=D");
        assert_eq!(plain("a\nb\u{e9}"), "a\\nb\\u{e9}");
    }

    #[test]
    fn code_is_fenced_past_its_own_backticks() {
        assert_eq!(code("ClOrdID"), "`ClOrdID`");
        assert_eq!(code("a`b"), "`` a`b ``");
        assert_eq!(code("a``b"), "``` a``b ```");
    }

    #[test]
    fn an_alias_names_the_fix_name() {
        assert_eq!(
            alias("ClOrdID", "cl_ord_id").as_deref(),
            Some("#[doc(alias = \"ClOrdID\")]")
        );
    }

    #[test]
    fn an_item_named_like_the_field_has_no_alias() {
        assert_eq!(alias("symbol", "symbol"), None);
        assert_eq!(alias("type", "r#type"), None);
    }

    #[test]
    fn a_name_that_is_not_a_word_has_no_alias() {
        assert_eq!(alias("Cl\"OrdID", "cl_ord_id"), None);
        assert_eq!(alias("", "x"), None);
    }
}
