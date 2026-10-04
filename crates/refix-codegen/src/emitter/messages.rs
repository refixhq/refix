use refix_dictionary::{DataType, Field, MemberContext, dictionary};

use super::Error;
use super::groups::{emit_group_accessor, emit_group_module, emit_known_tags, known_tags};
use super::layout::{MAX_WIDTH, indent};
use super::naming::{message_module_name, method_name};

pub(super) fn emit_message(message: dictionary::Message<'_>) -> Result<String, Error> {
    let mut items = vec![emit_message_struct(message), emit_message_impl(message)?];
    if let Some(module) = emit_message_module(message)? {
        items.push(module);
    }
    Ok(items.join("\n"))
}

fn emit_message_struct(message: dictionary::Message<'_>) -> String {
    format!("pub struct {}(RawMessage);\n", message.name())
}

fn emit_message_impl(message: dictionary::Message<'_>) -> Result<String, Error> {
    let mut members = vec![
        format!(
            "    pub const MSG_TYPE: &[u8] = b\"{}\";\n",
            message.msg_type()
        ),
        indent(
            &emit_known_tags(&known_tags(message.members()), MAX_WIDTH - 4),
            1,
        ),
        "    pub fn from_raw(raw: RawMessage) -> Self {\n        Self(raw)\n    }\n".to_owned(),
        "    pub fn raw(&self) -> &RawMessage {\n        &self.0\n    }\n".to_owned(),
    ];
    for member in message.members() {
        match member {
            dictionary::Member::Field { field, .. } => {
                members.push(emit_accessor(field, Lifetime::Receiver)?)
            }
            dictionary::Member::Group(group) => members.push(emit_group_accessor(
                group,
                Lifetime::Receiver,
                "Self::KNOWN_TAGS",
            )?),
        }
    }

    Ok(format!(
        "impl {} {{\n{}}}\n",
        message.name(),
        members.join("\n")
    ))
}

/// The module holding the groups declared directly in a message, if it
/// declares any.
fn emit_message_module(message: dictionary::Message<'_>) -> Result<Option<String>, Error> {
    let modules = message
        .members()
        .filter_map(|member| match member {
            dictionary::Member::Group(group)
                if matches!(group.declared_in(), MemberContext::Message(_)) =>
            {
                Some(emit_group_module(group))
            }
            _ => None,
        })
        .collect::<Result<Vec<_>, _>>()?;
    if modules.is_empty() {
        return Ok(None);
    }
    Ok(Some(format!(
        "pub mod {} {{\n{}}}\n",
        message_module_name(message.name())?,
        indent(&modules.join("\n"), 1)
    )))
}

/// The lifetime an accessor's borrowed results live for.
#[derive(Clone, Copy)]
pub(super) enum Lifetime {
    /// The receiver's elided lifetime, as a message owns its bytes.
    Receiver,
    /// The wrapped scope's `'a`, as a group instance borrows its bytes.
    Scope,
}

pub(super) fn emit_accessor(field: &Field, lifetime: Lifetime) -> Result<String, Error> {
    let name = method_name(field)?;
    let tag = field.tag;
    let (reference, type_lifetime) = match lifetime {
        Lifetime::Receiver => ("&", "'_"),
        Lifetime::Scope => ("&'a ", "'a"),
    };

    if !field.values.is_empty() {
        let type_name = &field.name;
        if field.data_type.is_multiple_value() {
            return Ok(format!(
                "    pub fn {name}(&self) -> Result<Option<MultipleValues<{type_lifetime}, {type_name}<{type_lifetime}>>>, InvalidValue> {{\n        self.0.get_multiple_values(Tag({tag}))\n    }}\n"
            ));
        }
        let (conversion, enum_type) = if field.data_type.is_int_based() {
            ("get_int", type_name.to_owned())
        } else {
            ("get_str", format!("{type_name}<{type_lifetime}>"))
        };
        return Ok(format!(
            "    pub fn {name}(&self) -> Result<Option<{enum_type}>, InvalidValue> {{\n        Ok(self.0.{conversion}(Tag({tag}))?.map({type_name}::from_value))\n    }}\n"
        ));
    }

    let accessor = match &field.data_type {
        DataType::String => format!(
            "    pub fn {name}(&self) -> Result<Option<{reference}str>, InvalidValue> {{\n        self.0.get_str(Tag({tag}))\n    }}\n"
        ),
        DataType::Int => format!(
            "    pub fn {name}(&self) -> Result<Option<i64>, InvalidValue> {{\n        self.0.get_int(Tag({tag}))\n    }}\n"
        ),
        _ => format!(
            "    pub fn {name}(&self) -> Option<{reference}[u8]> {{\n        self.0.get(Tag({tag}))\n    }}\n"
        ),
    };

    Ok(accessor)
}

#[cfg(test)]
mod tests {
    use super::{Lifetime, emit_accessor};
    use refix_dictionary::{DataType, EnumValue, Field, Tag};

    fn field(name: &str, tag: u32, data_type: DataType) -> Field {
        Field {
            name: name.to_owned(),
            tag: Tag(tag),
            data_type,
            values: vec![],
        }
    }

    fn scope_signature(field: &Field) -> String {
        let accessor = emit_accessor(field, Lifetime::Scope).unwrap();
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
        let accessor = emit_accessor(&exec_inst, Lifetime::Receiver).unwrap();

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
            let accessor = emit_accessor(field, Lifetime::Receiver).unwrap();
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
