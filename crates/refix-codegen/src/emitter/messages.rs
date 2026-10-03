use refix_dictionary::{DataType, Field, MemberContext, dictionary};

use super::{Error, Warning, naming::method_name};

pub(super) fn emit_message(
    message: dictionary::Message<'_>,
    warnings: &mut Vec<Warning>,
) -> Result<String, Error> {
    let message_struct = emit_message_struct(message);
    let message_impl = emit_message_impl(message, warnings)?;
    Ok(format!("{message_struct}\n{message_impl}"))
}

fn emit_message_struct(message: dictionary::Message<'_>) -> String {
    format!("pub struct {}(RawMessage);\n", message.name())
}

fn emit_message_impl(
    message: dictionary::Message<'_>,
    warnings: &mut Vec<Warning>,
) -> Result<String, Error> {
    let mut members = vec![
        format!(
            "    pub const MSG_TYPE: &[u8] = b\"{}\";\n",
            message.msg_type()
        ),
        "    pub fn from_raw(raw: RawMessage) -> Self {\n        Self(raw)\n    }\n".to_owned(),
        "    pub fn raw(&self) -> &RawMessage {\n        &self.0\n    }\n".to_owned(),
    ];
    for member in message.members() {
        match member {
            dictionary::Member::Field { field, .. } => {
                members.push(emit_accessor(field, Lifetime::Receiver)?)
            }
            dictionary::Member::Group(group) => {
                let context = MemberContext::Message(message.name().to_owned());
                warnings.push(Warning::UnsupportedGroup {
                    context: context.group(&group.count_field().name),
                });
            }
        }
    }

    Ok(format!(
        "impl {} {{\n{}}}\n",
        message.name(),
        members.join("\n")
    ))
}

/// The lifetime an accessor's borrowed results live for.
#[derive(Clone, Copy)]
enum Lifetime {
    /// The receiver's, elided: a message owns its bytes.
    Receiver,
    /// The wrapped scope's `'a`: a group instance borrows its bytes.
    Scope,
}

fn emit_accessor(field: &Field, lifetime: Lifetime) -> Result<String, Error> {
    let name = method_name(field)?;
    let tag = field.tag;
    let (reference, type_lifetime) = match lifetime {
        Lifetime::Receiver => ("&", "'_"),
        Lifetime::Scope => ("&'a ", "'a"),
    };

    if !field.values.is_empty() {
        let type_name = &field.name;
        return Ok(format!(
            "    pub fn {name}(&self) -> Option<{type_name}<{type_lifetime}>> {{\n        self.0.get(Tag({tag})).map({type_name}::from_bytes)\n    }}\n"
        ));
    }

    let accessor = match &field.data_type {
        DataType::String => format!(
            "    pub fn {name}(&self) -> Result<Option<{reference}str>, InvalidValue> {{\n        self.0.get_str(Tag({tag}))\n    }}\n"
        ),
        DataType::Int => format!(
            "    pub fn {name}(&self) -> Result<Option<i64>, InvalidValue> {{\n        self.0.get_int(Tag({tag}))\n    }}\n"
        ),
        DataType::Other(_) => format!(
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

    #[test]
    fn a_scope_enum_borrows_for_the_scope() {
        let mut party_role = field("PartyRole", 452, DataType::Int);
        party_role.values = vec![EnumValue {
            value: "1".to_owned(),
            description: "EXECUTING_FIRM".to_owned(),
        }];
        assert_eq!(
            scope_signature(&party_role),
            "pub fn party_role(&self) -> Option<PartyRole<'a>> {"
        );
    }
}
