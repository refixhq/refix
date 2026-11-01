use refix_dictionary::Dictionary;

use super::layout::MAX_WIDTH;

/// The `Message` alias and `from_raw`, which reads a raw message as the class
/// its MsgType names. A dictionary without messages has neither.
pub(super) fn emit_dispatch(dictionary: &Dictionary) -> Option<String> {
    let names: Vec<&str> = dictionary
        .messages()
        .map(|message| message.name())
        .collect();
    if names.is_empty() {
        return None;
    }
    let one_line = format!("type Message = {}", names.join(" | "));
    let alias = if one_line.len() <= MAX_WIDTH {
        format!("{one_line}\n")
    } else {
        format!("type Message = (\n    {}\n)\n", names.join("\n    | "))
    };
    let entries: String = names
        .iter()
        .map(|name| format!("    {name}.MSG_TYPE: {name},\n"))
        .collect();
    let messages = format!("_MESSAGES: dict[bytes, type[Message]] = {{\n{entries}}}\n");
    let from_raw = "def from_raw(raw: RawMessage) -> Message | RawMessage:\n    msg_type = raw.get(35)\n    message = None if msg_type is None else _MESSAGES.get(msg_type)\n    return raw if message is None else message(raw)\n";
    Some([alias.as_str(), &messages, from_raw].join("\n\n"))
}
