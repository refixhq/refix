use crate::snake_case;
use refix_dictionary::{DataType, Dictionary, Field, Message};
use std::collections::HashMap;

pub fn generate(dictionary: &Dictionary) -> Result<String, Error> {
    let fields_by_tag: HashMap<u32, &Field> = dictionary
        .fields
        .iter()
        .map(|field| (field.tag, field))
        .collect();
    let messages: Vec<String> = dictionary
        .messages
        .iter()
        .map(|message| emit_message(message, &fields_by_tag))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(messages.join("\n"))
}

fn emit_message(message: &Message, fields_by_tag: &HashMap<u32, &Field>) -> Result<String, Error> {
    let message_struct = emit_message_struct(message);
    let message_impl = emit_message_impl(message, fields_by_tag)?;
    Ok(format!("{message_struct}\n{message_impl}"))
}

fn emit_message_struct(message: &Message) -> String {
    let mut code = "struct ".to_owned();
    code.push_str(&message.name);
    code.push_str("(RawMessage);");

    code
}

fn emit_message_impl(
    message: &Message,
    fields_by_tag: &HashMap<u32, &Field>,
) -> Result<String, Error> {
    let mut code = "impl ".to_owned();
    code.push_str(&message.name);
    code.push_str(" {\n");
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
    let accessors: Vec<String> = fields
        .iter()
        .map(|field_ref| {
            let field = fields_by_tag.get(&field_ref.tag).unwrap();
            emit_accessor(field)
        })
        .collect();
    code.push_str(&accessors.join("\n"));
    code.push_str("}\n");

    Ok(code)
}

fn emit_accessor(field: &Field) -> String {
    let mut code = "fn ".to_owned();
    code.push_str(&snake_case(&field.name));
    code.push_str("(&self) -> ");
    code.push_str(&data_type_to_rust_type(&field.data_type));
    code.push_str(" {\n");
    code.push_str("self.0.get(");
    code.push_str(field.tag.to_string().as_str());
    code.push_str(")\n}");

    code
}

fn data_type_to_rust_type(data_type: &DataType) -> String {
    match data_type {
        DataType::String => "Result<Option<&str>, InvalidValue>".to_owned(),
        DataType::Int => "Result<Option<i64>, InvalidValue>".to_owned(),
        DataType::Other(_) => "Option<&[u8]>".to_owned(),
    }
}

#[derive(Debug)]
pub enum Error {
    UnknownTag { message: String, tag: u32 },
}
