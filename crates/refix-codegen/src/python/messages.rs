use refix_dictionary::{MemberContext, dictionary};

use super::declarations::emit_declarations;
use super::enums::{bytes_literal, docstring};
use super::groups::emit_group_class;
use super::layout::indent;
use super::names::Names;
use super::views::emit_view_declaration;
use crate::envelope::Envelope;
use crate::flat;

/// A message's class, with the classes of the groups the message declares
/// itself nested in it. It reaches each view the `envelope` has.
pub(super) fn emit_message(
    message: dictionary::Message<'_>,
    envelope: &Envelope,
    names: &Names,
) -> String {
    let name = message.name();
    let summary = format!("The {name} (35={}) message.", message.msg_type());
    let mut items = vec![
        format!("    {}\n", docstring(&summary)),
        format!(
            "    MSG_TYPE: ClassVar[bytes] = {}\n",
            bytes_literal(message.msg_type())
        ),
    ];
    items.extend(
        flat::members(message.members()).filter_map(|member| match member {
            flat::Member::Group(group)
                if matches!(group.declared_in(), MemberContext::Message(_)) =>
            {
                Some(indent(&emit_group_class(group, names), 1))
            }
            _ => None,
        }),
    );
    let views: String = envelope
        .sections
        .iter()
        .map(|section| emit_view_declaration(*section))
        .collect();
    let declarations = views + &emit_declarations(message.members(), names);
    if !declarations.is_empty() {
        items.push(declarations);
    }
    format!("class {name}(fields.Message):\n{}", items.join("\n"))
}
