use refix_dictionary::{DataType, Field, dictionary};

use crate::snake_case;

/// A field accessor's name before escaping.
///
/// A field of a type the generator does not interpret takes the `_raw` suffix.
pub(crate) fn field_base_name(field: &Field) -> String {
    let mut name = snake_case(&field.name);
    let interpreted = matches!(field.data_type, DataType::String | DataType::Char)
        || field.data_type.is_int_based();
    if field.values.is_empty() && !interpreted {
        name.push_str("_raw");
    }
    name
}

/// The accessor and module name of a group, before escaping.
pub(crate) fn group_base_name(group: dictionary::Group<'_>) -> String {
    snake_case(group_source_name(group))
}

/// A group's name as the dictionary gives it, before snake-casing.
///
/// The component it makes up, else its count field's name without the `No` prefix,
/// else its count field's name.
pub(crate) fn group_source_name(group: dictionary::Group<'_>) -> &str {
    let count_field = group.count_field().name.as_str();
    group
        .component()
        .or_else(|| without_no_prefix(count_field))
        .unwrap_or(count_field)
}

/// Strips a `No` that starts a word, as in `NoPartyIDs` but not `Notional`.
fn without_no_prefix(name: &str) -> Option<&str> {
    name.strip_prefix("No")
        .filter(|rest| rest.starts_with(|c: char| c.is_ascii_uppercase()))
}

#[cfg(test)]
mod tests {
    use super::{field_base_name, group_base_name, group_source_name};
    use crate::test_utils::with_group;
    use refix_dictionary::{DataType, EnumValue, Field, Tag};

    fn field(name: &str, data_type: DataType) -> Field {
        Field {
            name: name.to_owned(),
            tag: Tag(1),
            data_type,
            values: vec![],
        }
    }

    fn group_named(component: Option<&str>, count_field: &str) -> String {
        with_group(component, count_field, group_base_name)
    }

    #[test]
    fn field_names_are_snake_case() {
        assert_eq!(
            field_base_name(&field("ClOrdID", DataType::String)),
            "cl_ord_id"
        );
    }

    #[test]
    fn an_unclaimed_type_takes_the_raw_suffix() {
        let price = field("Price", DataType::Other("PRICE".to_owned()));
        assert_eq!(field_base_name(&price), "price_raw");
    }

    #[test]
    fn an_int_based_type_takes_the_plain_name() {
        let msg_seq_num = field("MsgSeqNum", DataType::SeqNum);
        assert_eq!(field_base_name(&msg_seq_num), "msg_seq_num");
    }

    #[test]
    fn a_char_takes_the_plain_name() {
        let opt_attribute = field("OptAttribute", DataType::Char);
        assert_eq!(field_base_name(&opt_attribute), "opt_attribute");
    }

    #[test]
    fn an_unclaimed_type_takes_the_suffix_even_when_modelled() {
        let exec_inst = field("ExecInst", DataType::MultipleStringValue);
        assert_eq!(field_base_name(&exec_inst), "exec_inst_raw");
    }

    #[test]
    fn an_enum_field_takes_the_plain_name() {
        let mut ord_type = field("OrdType", DataType::Char);
        ord_type.values = vec![EnumValue {
            value: "1".to_owned(),
            description: "MARKET".to_owned(),
        }];
        assert_eq!(field_base_name(&ord_type), "ord_type");
    }

    #[test]
    fn a_source_name_keeps_the_dictionary_spelling() {
        let source_name = |component, count_field| {
            with_group(component, count_field, |group| {
                group_source_name(group).to_owned()
            })
        };
        assert_eq!(source_name(Some("Parties"), "NoPartyIDs"), "Parties");
        assert_eq!(source_name(None, "NoMsgTypes"), "MsgTypes");
    }

    #[test]
    fn a_group_takes_the_name_of_its_component() {
        assert_eq!(group_named(Some("Parties"), "NoPartyIDs"), "parties");
    }

    #[test]
    fn an_inline_group_drops_the_no_prefix() {
        assert_eq!(group_named(None, "NoMsgTypes"), "msg_types");
    }

    #[test]
    fn a_no_that_does_not_start_a_word_is_kept() {
        assert_eq!(group_named(None, "Nominees"), "nominees");
    }

    #[test]
    fn a_count_field_without_the_prefix_is_used_as_is() {
        assert_eq!(group_named(None, "LegCount"), "leg_count");
    }
}
