/// Converts a FIX PascalCase name to a snake_case identifier.
///
/// Acronym runs stay together, with the last capital starting the next
/// word when lowercase follows (`ClOrdID` -> `cl_ord_id`, `MDEntryPx` ->
/// `md_entry_px`), and a lone `s` stays with the acronym it pluralises
/// (`NoPartyIDs` -> `no_party_ids`). Digits attach to the preceding word
/// (`Nested2PartyID` -> `nested2_party_id`).
pub fn snake_case(name: &str) -> String {
    let mut converted = String::with_capacity(name.len() + 4);
    let mut chars = name.chars().peekable();
    let mut previous: Option<char> = None;

    while let Some(current) = chars.next() {
        if current.is_ascii_uppercase() {
            let after_word = previous.is_some_and(|p| p.is_ascii_lowercase() || p.is_ascii_digit());
            let ends_acronym = previous.is_some_and(|p| p.is_ascii_uppercase())
                && chars.peek().is_some_and(|n| n.is_ascii_lowercase())
                && !pluralizes(chars.clone());
            if after_word || ends_acronym {
                converted.push('_');
            }
            converted.push(current.to_ascii_lowercase());
        } else {
            converted.push(current);
        }
        previous = Some(current);
    }

    converted
}

/// Whether the rest of a name opens with a lone `s`, the plural of the
/// acronym before it.
fn pluralizes(mut rest: impl Iterator<Item = char>) -> bool {
    rest.next() == Some('s') && !rest.next().is_some_and(|c| c.is_ascii_lowercase())
}

/// Converts a UPPER_CASE_SNAKE enum value description to a PascalCase
/// identifier.
///
/// Underscores are the word boundaries, so acronyms flatten
/// (`IOI_QTY` -> `IoiQty`).
/// Empty segments from doubled underscores are skipped.
pub fn pascal_case(name: &str) -> String {
    let mut converted = String::with_capacity(name.len());

    for segment in name.split('_').filter(|segment| !segment.is_empty()) {
        let mut chars = segment.chars();
        if let Some(first) = chars.next() {
            converted.push(first.to_ascii_uppercase());
        }
        for rest in chars {
            converted.push(rest.to_ascii_lowercase());
        }
    }

    converted
}

#[cfg(test)]
mod tests {
    use super::{pascal_case, snake_case};

    #[test]
    fn lowercases_a_single_word() {
        assert_eq!(snake_case("Account"), "account");
    }

    #[test]
    fn splits_words_at_case_changes() {
        assert_eq!(snake_case("OrderQty"), "order_qty");
    }

    #[test]
    fn keeps_a_trailing_acronym_together() {
        assert_eq!(snake_case("ClOrdID"), "cl_ord_id");
    }

    #[test]
    fn keeps_a_name_that_is_one_acronym_run_together() {
        assert_eq!(snake_case("IOIID"), "ioiid");
    }

    #[test]
    fn splits_a_leading_acronym_before_its_last_capital() {
        assert_eq!(snake_case("MDEntryPx"), "md_entry_px");
    }

    #[test]
    fn splits_an_acronym_in_the_middle_of_a_name() {
        assert_eq!(snake_case("NoMDEntries"), "no_md_entries");
    }

    #[test]
    fn splits_after_an_acronym_at_the_start() {
        assert_eq!(snake_case("XMLData"), "xml_data");
    }

    #[test]
    fn keeps_a_plural_acronym_together() {
        assert_eq!(snake_case("NoPartyIDs"), "no_party_ids");
    }

    #[test]
    fn splits_a_plural_acronym_from_the_next_word() {
        assert_eq!(snake_case("PartyIDsSource"), "party_ids_source");
    }

    #[test]
    fn attaches_digits_to_the_preceding_word() {
        assert_eq!(snake_case("Nested2PartyID"), "nested2_party_id");
    }

    #[test]
    fn capitalizes_a_single_word() {
        assert_eq!(pascal_case("BUY"), "Buy");
    }

    #[test]
    fn joins_words_at_underscores() {
        assert_eq!(pascal_case("GOOD_TILL_CANCEL"), "GoodTillCancel");
    }

    #[test]
    fn flattens_acronym_words() {
        assert_eq!(pascal_case("IOI_QTY"), "IoiQty");
    }

    #[test]
    fn keeps_digits_within_words() {
        assert_eq!(pascal_case("FILL_OR_KILL_4"), "FillOrKill4");
    }

    #[test]
    fn passes_digit_only_words_through() {
        assert_eq!(pascal_case("FIX_4_4"), "Fix44");
    }

    #[test]
    fn skips_empty_segments() {
        assert_eq!(pascal_case("AT__THE_OPENING"), "AtTheOpening");
    }

    #[test]
    fn normalizes_lowercase_input() {
        assert_eq!(pascal_case("buy_minus"), "BuyMinus");
    }
}
