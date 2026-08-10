use refix_dictionary::{DataType, Field, Message};
use std::collections::HashMap;

use super::{Error, naming::method_name};

pub(super) fn emit_message(
    message: &Message,
    fields_by_tag: &HashMap<u32, &Field>,
) -> Result<String, Error> {
    let message_struct = emit_message_struct(message);
    let message_impl = emit_message_impl(message, fields_by_tag)?;
    Ok(format!("{message_struct}\n{message_impl}"))
}

fn emit_message_struct(message: &Message) -> String {
    format!("pub struct {}(RawMessage);\n", message.name)
}

fn emit_message_impl(
    message: &Message,
    fields_by_tag: &HashMap<u32, &Field>,
) -> Result<String, Error> {
    let fields = message
        .fields
        .iter()
        .map(|field_ref| {
            fields_by_tag
                .get(&field_ref.tag)
                .copied()
                .ok_or(Error::UnknownTag {
                    message: message.name.clone(),
                    tag: field_ref.tag,
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut members = vec![
        format!(
            "    pub const MSG_TYPE: &[u8] = b\"{}\";\n",
            message.msg_type
        ),
        "    pub fn from_raw(raw: RawMessage) -> Self {\n        Self(raw)\n    }\n".to_owned(),
        "    pub fn raw(&self) -> &RawMessage {\n        &self.0\n    }\n".to_owned(),
    ];
    members.extend(
        fields
            .into_iter()
            .map(emit_accessor)
            .collect::<Result<Vec<_>, _>>()?,
    );

    Ok(format!(
        "impl {} {{\n{}}}\n",
        message.name,
        members.join("\n")
    ))
}

fn emit_accessor(field: &Field) -> Result<String, Error> {
    let name = method_name(field)?;
    let tag = field.tag;

    if !field.values.is_empty() {
        let type_name = &field.name;
        return Ok(format!(
            "    pub fn {name}(&self) -> Option<{type_name}<'_>> {{\n        self.0.get({tag}).map({type_name}::from_bytes)\n    }}\n"
        ));
    }

    let accessor = match &field.data_type {
        DataType::String => format!(
            "    pub fn {name}(&self) -> Result<Option<&str>, InvalidValue> {{\n        self.0.get_str({tag})\n    }}\n"
        ),
        DataType::Int => format!(
            "    pub fn {name}(&self) -> Result<Option<i64>, InvalidValue> {{\n        self.0.get_int({tag})\n    }}\n"
        ),
        DataType::Other(_) => format!(
            "    pub fn {name}(&self) -> Option<&[u8]> {{\n        self.0.get({tag})\n    }}\n"
        ),
    };

    Ok(accessor)
}
