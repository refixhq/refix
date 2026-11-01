use refix_dictionary::Dictionary;

/// The `Message` enum, which reads a raw message as the message its MsgType
/// names. A dictionary without messages has none.
pub(super) fn emit_dispatch(dictionary: &Dictionary) -> Option<String> {
    let names: Vec<&str> = dictionary
        .messages()
        .map(|message| message.name())
        .collect();
    if names.is_empty() {
        return None;
    }
    let variants: String = names
        .iter()
        .map(|name| format!("    {name}({name}),\n"))
        .collect();
    let arms: String = names
        .iter()
        .map(|name| {
            format!("            Some({name}::MSG_TYPE) => Self::{name}({name}::from_raw(raw)),\n")
        })
        .collect();
    Some(format!(
        "pub enum Message {{\n{variants}    Unrecognized(RawMessage),\n}}\n\nimpl Message {{\n    pub fn from_raw(raw: RawMessage) -> Self {{\n        match raw.get(Tag(35)) {{\n{arms}            _ => Self::Unrecognized(raw),\n        }}\n    }}\n}}\n"
    ))
}
