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
            dictionary::Member::Field { field, .. } => members.push(emit_accessor(field)?),
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

fn emit_accessor(field: &Field) -> Result<String, Error> {
    let name = method_name(field)?;
    let tag = field.tag;

    if !field.values.is_empty() {
        let type_name = &field.name;
        return Ok(format!(
            "    pub fn {name}(&self) -> Option<{type_name}<'_>> {{\n        self.0.get(Tag({tag})).map({type_name}::from_bytes)\n    }}\n"
        ));
    }

    let accessor = match &field.data_type {
        DataType::String => format!(
            "    pub fn {name}(&self) -> Result<Option<&str>, InvalidValue> {{\n        self.0.get_str(Tag({tag}))\n    }}\n"
        ),
        DataType::Int => format!(
            "    pub fn {name}(&self) -> Result<Option<i64>, InvalidValue> {{\n        self.0.get_int(Tag({tag}))\n    }}\n"
        ),
        DataType::Other(_) => format!(
            "    pub fn {name}(&self) -> Option<&[u8]> {{\n        self.0.get(Tag({tag}))\n    }}\n"
        ),
    };

    Ok(accessor)
}
