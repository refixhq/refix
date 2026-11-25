use refix_dictionary::Dictionary;

use super::groups::known_tags_of;
use super::messages::walks_groups;
use super::names::Names;
use crate::components::components;
use crate::envelope::Envelope;
use crate::groups::declared_groups;

/// The `warm_up` function, which builds every known tags list.
///
/// A dictionary without such a list has none.
pub(super) fn emit_warm_up(
    dictionary: &Dictionary,
    envelope: &Envelope,
    names: &Names,
) -> Option<String> {
    let mut lists = Vec::new();
    for section in &envelope.sections {
        if envelope.with_groups.contains(section) {
            lists.push(format!("{}::envelope_tags()", section.type_name()));
        }
        lists.push(known_tags_of(section.type_name()));
    }
    for component in components(dictionary) {
        lists.push(known_tags_of(component.name()));
    }
    for group in declared_groups(dictionary) {
        lists.push(known_tags_of(names.group_type(group)));
    }
    for message in dictionary.messages() {
        if walks_groups(message, envelope) {
            lists.push(known_tags_of(message.name()));
        }
    }
    if lists.is_empty() {
        return None;
    }
    let calls: String = lists
        .iter()
        .map(|list| format!("    {list}.get();\n"))
        .collect();
    Some(format!(
        "/// Builds the tag lists that group reads share.\n///\n/// Each list is otherwise built by the first read that needs it. Call this\n/// once at startup to keep that work out of the first messages.\npub fn warm_up() {{\n{calls}}}\n"
    ))
}
