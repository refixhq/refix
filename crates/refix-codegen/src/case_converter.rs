/// Converts a FIX PascalCase name to a snake_case identifier.
///
/// Acronym runs stay together, with the last capital starting the next
/// word when lowercase follows (`ClOrdID` -> `cl_ord_id`, `MDEntryPx` ->
/// `md_entry_px`). Digits attach to the preceding word (`Nested2PartyID`
/// -> `nested2_party_id`).
pub fn snake_case(name: &str) -> String {
    let mut converted = String::with_capacity(name.len() + 4);
    let mut chars = name.chars().peekable();
    let mut previous: Option<char> = None;

    while let Some(current) = chars.next() {
        if current.is_ascii_uppercase() {
            let after_word = previous.is_some_and(|p| p.is_ascii_lowercase() || p.is_ascii_digit());
            let ends_acronym = previous.is_some_and(|p| p.is_ascii_uppercase())
                && chars.peek().is_some_and(|n| n.is_ascii_lowercase());
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

#[cfg(test)]
mod tests {
    use super::snake_case;

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
    fn attaches_digits_to_the_preceding_word() {
        assert_eq!(snake_case("Nested2PartyID"), "nested2_party_id");
    }
}
