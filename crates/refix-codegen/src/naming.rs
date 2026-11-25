use refix_dictionary::dictionary;
use refix_dictionary::spec::Field;

use crate::conversion::Conversion;
use crate::snake_case;

/// A field accessor's name before escaping.
///
/// A field of a type the generator does not interpret takes the `_raw` suffix.
pub(crate) fn field_base_name(field: &Field) -> String {
    let mut name = snake_case(&field.name);
    if Conversion::of(field) == Conversion::Raw {
        name.push_str("_raw");
    }
    name
}

/// The accessor of a group, before escaping.
///
/// Its count field's name without the `No` prefix, else its count field's name.
pub(crate) fn group_accessor_base_name(group: dictionary::Group<'_>) -> String {
    let count_field = group.count_field().name.as_str();
    snake_case(without_no_prefix(count_field).unwrap_or(count_field))
}

/// A group's name as the dictionary gives it.
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

/// The module holding the type named `type_name`, before escaping.
///
/// It is the name in snake case, unless that splits an acronym in the wrong
/// place.
pub(crate) fn module_base_name(type_name: &str) -> String {
    match MODULE_EXCEPTIONS
        .iter()
        .find(|(name, _)| *name == type_name)
    {
        Some((_, module)) => (*module).to_owned(),
        None => snake_case(type_name),
    }
}

/// Types whose snake-case name splits an acronym in the wrong place.
const MODULE_EXCEPTIONS: &[(&str, &str)] = &[("XMLnonFIX", "xml_non_fix")];

/// Strips a `No` that starts a word, as in `NoPartyIDs` but not `Notional`.
fn without_no_prefix(name: &str) -> Option<&str> {
    name.strip_prefix("No")
        .filter(|rest| rest.starts_with(|c: char| c.is_ascii_uppercase()))
}

#[cfg(test)]
mod tests {
    use super::{field_base_name, group_accessor_base_name, group_source_name, module_base_name};
    use crate::test_utils::with_group;
    use refix_dictionary::Tag;
    use refix_dictionary::spec::{DataType, EnumValue, Field};

    fn field(name: &str, data_type: DataType) -> Field {
        Field {
            name: name.to_owned(),
            tag: Tag(1),
            data_type,
            length_tag: None,
            values: vec![],
        }
    }

    fn accessor_named(component: Option<&str>, count_field: &str) -> String {
        with_group(component, count_field, group_accessor_base_name)
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
        let tz_transact_time = field("TZTransactTime", DataType::TzTimestamp);
        assert_eq!(field_base_name(&tz_transact_time), "tz_transact_time_raw");
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
    fn an_accessor_takes_the_count_field_without_the_no_prefix() {
        assert_eq!(accessor_named(Some("Parties"), "NoPartyIDs"), "party_ids");
        assert_eq!(accessor_named(None, "NoMsgTypes"), "msg_types");
    }

    #[test]
    fn a_no_that_does_not_start_a_word_is_kept() {
        assert_eq!(accessor_named(Some("Parties"), "Nominees"), "nominees");
    }

    #[test]
    fn a_count_field_without_the_prefix_is_used_as_is() {
        assert_eq!(accessor_named(Some("Parties"), "LegCount"), "leg_count");
    }

    #[test]
    fn module_names_are_snake_case() {
        assert_eq!(module_base_name("NewOrderSingle"), "new_order_single");
        assert_eq!(module_base_name("SecAltIDGrp"), "sec_alt_id_grp");
    }

    #[test]
    fn a_module_name_that_would_split_an_acronym_is_listed() {
        assert_eq!(module_base_name("XMLnonFIX"), "xml_non_fix");
    }
}
