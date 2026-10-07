use refix_dictionary::{DataType, Field, MemberContext, dictionary};

use super::groups::{emit_group_accessor, emit_group_module, emit_known_tags};
use super::layout::{MAX_WIDTH, indent};
use super::names::Names;
use super::views::emit_view_accessor;
use crate::envelope::Envelope;
use crate::groups::message_known_tags;

/// A message's struct, impl and group module. Its known tags include the
/// `envelope`'s, and it reaches each view the envelope has.
pub(super) fn emit_message(
    message: dictionary::Message<'_>,
    envelope: &Envelope,
    names: &Names,
) -> String {
    let mut items = vec![
        emit_message_struct(message),
        emit_message_impl(message, envelope, names),
    ];
    if let Some(module) = emit_message_module(message, names) {
        items.push(module);
    }
    items.join("\n")
}

fn emit_message_struct(message: dictionary::Message<'_>) -> String {
    format!("pub struct {}(RawMessage);\n", message.name())
}

fn emit_message_impl(
    message: dictionary::Message<'_>,
    envelope: &Envelope,
    names: &Names,
) -> String {
    let mut members = vec![
        format!(
            "    pub const MSG_TYPE: &[u8] = b\"{}\";\n",
            message.msg_type()
        ),
        indent(
            &emit_known_tags(&message_known_tags(message, &envelope.tags), MAX_WIDTH - 4),
            1,
        ),
        "    pub fn from_raw(raw: RawMessage) -> Self {\n        Self(raw)\n    }\n".to_owned(),
        "    pub fn raw(&self) -> &RawMessage {\n        &self.0\n    }\n".to_owned(),
    ];
    for section in &envelope.sections {
        members.push(emit_view_accessor(*section, envelope));
    }
    for member in message.members() {
        match member {
            dictionary::Member::Field { field, .. } => {
                members.push(emit_accessor(field, names, Lifetime::Receiver))
            }
            dictionary::Member::Group(group) => members.push(emit_group_accessor(
                group,
                names,
                Lifetime::Receiver,
                "Self::KNOWN_TAGS",
            )),
        }
    }

    format!("impl {} {{\n{}}}\n", message.name(), members.join("\n"))
}

/// The module holding the groups declared directly in a message, if it
/// declares any.
fn emit_message_module(message: dictionary::Message<'_>, names: &Names) -> Option<String> {
    let modules: Vec<String> = message
        .members()
        .filter_map(|member| match member {
            dictionary::Member::Group(group)
                if matches!(group.declared_in(), MemberContext::Message(_)) =>
            {
                Some(emit_group_module(group, names))
            }
            _ => None,
        })
        .collect();
    if modules.is_empty() {
        return None;
    }
    Some(format!(
        "pub mod {} {{\n{}}}\n",
        names.message_module(message.name()),
        indent(&modules.join("\n"), 1)
    ))
}

/// The lifetime an accessor's borrowed results live for.
#[derive(Clone, Copy)]
pub(super) enum Lifetime {
    /// The receiver's elided lifetime, as a message owns its bytes.
    Receiver,
    /// The wrapped scope's `'a`, as a group instance borrows its bytes.
    Scope,
}

pub(super) fn emit_accessor(field: &Field, names: &Names, lifetime: Lifetime) -> String {
    let name = names.accessor(field);
    let tag = field.tag;
    let (reference, type_lifetime) = match lifetime {
        Lifetime::Receiver => ("&", "'_"),
        Lifetime::Scope => ("&'a ", "'a"),
    };

    if !field.values.is_empty() {
        let type_name = names.enum_type(field);
        if field.data_type.is_multiple_value() {
            return format!(
                "    pub fn {name}(&self) -> Result<Option<MultipleValues<{type_lifetime}, {type_name}<{type_lifetime}>>>, InvalidValue> {{\n        self.0.get_multiple_values(Tag({tag}))\n    }}\n"
            );
        }
        let (conversion, enum_type) = if field.data_type.is_int_based() {
            ("get_int", type_name.to_owned())
        } else {
            ("get_str", format!("{type_name}<{type_lifetime}>"))
        };
        return format!(
            "    pub fn {name}(&self) -> Result<Option<{enum_type}>, InvalidValue> {{\n        Ok(self.0.{conversion}(Tag({tag}))?.map({type_name}::from_value))\n    }}\n"
        );
    }

    match &field.data_type {
        DataType::String => format!(
            "    pub fn {name}(&self) -> Result<Option<{reference}str>, InvalidValue> {{\n        self.0.get_str(Tag({tag}))\n    }}\n"
        ),
        data_type if data_type.is_int_based() => format!(
            "    pub fn {name}(&self) -> Result<Option<i64>, InvalidValue> {{\n        self.0.get_int(Tag({tag}))\n    }}\n"
        ),
        _ => format!(
            "    pub fn {name}(&self) -> Option<{reference}[u8]> {{\n        self.0.get(Tag({tag}))\n    }}\n"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::{Lifetime, emit_accessor};
    use crate::rust::names::Names;
    use crate::test_utils::message_with;
    use refix_dictionary::{DataType, EnumValue, Field, Tag};

    fn field(name: &str, tag: u32, data_type: DataType) -> Field {
        Field {
            name: name.to_owned(),
            tag: Tag(tag),
            data_type,
            values: vec![],
        }
    }

    fn accessor(field: &Field, lifetime: Lifetime) -> String {
        let dictionary = message_with(vec![field.clone()]);
        emit_accessor(
            field,
            &Names::new(&dictionary, &mut Vec::new()).unwrap(),
            lifetime,
        )
    }

    fn scope_signature(field: &Field) -> String {
        let accessor = accessor(field, Lifetime::Scope);
        accessor.lines().next().unwrap().trim().to_owned()
    }

    #[test]
    fn a_scope_string_borrows_for_the_scope() {
        assert_eq!(
            scope_signature(&field("PartyID", 448, DataType::String)),
            "pub fn party_id(&self) -> Result<Option<&'a str>, InvalidValue> {"
        );
    }

    #[test]
    fn an_int_based_type_reads_as_an_integer() {
        assert_eq!(
            scope_signature(&field("MsgSeqNum", 34, DataType::SeqNum)),
            "pub fn msg_seq_num(&self) -> Result<Option<i64>, InvalidValue> {"
        );
    }

    #[test]
    fn a_scope_raw_value_borrows_for_the_scope() {
        let price = field("Price", 44, DataType::Other("PRICE".to_owned()));
        assert_eq!(
            scope_signature(&price),
            "pub fn price_raw(&self) -> Option<&'a [u8]> {"
        );
    }

    fn with_value(mut field: Field) -> Field {
        field.values = vec![EnumValue {
            value: "1".to_owned(),
            description: "FIRST".to_owned(),
        }];
        field
    }

    #[test]
    fn a_scope_enum_borrows_for_the_scope() {
        let ord_type = with_value(field("OrdType", 40, DataType::Char));
        assert_eq!(
            scope_signature(&ord_type),
            "pub fn ord_type(&self) -> Result<Option<OrdType<'a>>, InvalidValue> {"
        );
    }

    #[test]
    fn an_int_coded_enum_borrows_nothing() {
        let party_role = with_value(field("PartyRole", 452, DataType::Int));
        assert_eq!(
            scope_signature(&party_role),
            "pub fn party_role(&self) -> Result<Option<PartyRole>, InvalidValue> {"
        );
    }

    #[test]
    fn a_multiple_value_enum_reads_all_its_values() {
        let exec_inst = with_value(field("ExecInst", 18, DataType::MultipleStringValue));
        let accessor = accessor(&exec_inst, Lifetime::Receiver);

        assert_eq!(
            accessor,
            "    pub fn exec_inst(&self) -> Result<Option<MultipleValues<'_, ExecInst<'_>>>, InvalidValue> {\n        self.0.get_multiple_values(Tag(18))\n    }\n"
        );
    }

    #[test]
    fn a_scope_multiple_value_enum_borrows_for_the_scope() {
        let exec_inst = with_value(field("ExecInst", 18, DataType::MultipleCharValue));
        assert_eq!(
            scope_signature(&exec_inst),
            "pub fn exec_inst(&self) -> Result<Option<MultipleValues<'a, ExecInst<'a>>>, InvalidValue> {"
        );
    }

    #[test]
    fn a_multiple_value_field_without_values_stays_raw() {
        let exec_inst = field("ExecInst", 18, DataType::MultipleStringValue);
        assert_eq!(
            scope_signature(&exec_inst),
            "pub fn exec_inst_raw(&self) -> Option<&'a [u8]> {"
        );
    }

    #[test]
    fn an_enum_reads_through_its_base_type() {
        let party_role = with_value(field("PartyRole", 452, DataType::NumInGroup));
        let ord_type = with_value(field("OrdType", 40, DataType::Char));
        let body = |field: &Field| {
            let accessor = accessor(field, Lifetime::Receiver);
            accessor.lines().nth(1).unwrap().trim().to_owned()
        };

        assert_eq!(
            body(&party_role),
            "Ok(self.0.get_int(Tag(452))?.map(PartyRole::from_value))"
        );
        assert_eq!(
            body(&ord_type),
            "Ok(self.0.get_str(Tag(40))?.map(OrdType::from_value))"
        );
    }
}
