use std::collections::HashSet;

use refix_dictionary::{Dictionary, MemberContext, dictionary};

use crate::groups::instance_context;

/// A class the module defines once, for every message that uses it.
pub(super) enum Shared<'a> {
    /// A group declared in a component.
    Group(dictionary::Group<'a>),
    /// A component with fields.
    Component(dictionary::Component<'a>),
}

/// The shared classes the header, messages and trailer reach.
///
/// Each appears once, innermost first. A class is defined after the classes
/// its declarations name. Every other group's class sits where the group is
/// declared.
pub(super) fn shared_classes(dictionary: &Dictionary) -> Vec<Shared<'_>> {
    let mut classes = Vec::new();
    let mut seen = Seen::default();
    collect(dictionary.header(), &mut classes, &mut seen);
    for message in dictionary.messages() {
        collect(message.members(), &mut classes, &mut seen);
    }
    collect(dictionary.trailer(), &mut classes, &mut seen);
    classes
}

#[derive(Default)]
struct Seen<'a> {
    groups: HashSet<MemberContext>,
    components: HashSet<&'a str>,
}

fn collect<'a>(
    members: impl Iterator<Item = dictionary::Member<'a>>,
    classes: &mut Vec<Shared<'a>>,
    seen: &mut Seen<'a>,
) {
    for member in members {
        match member {
            dictionary::Member::Field { .. } => {}
            dictionary::Member::Group(group) => {
                if !seen.groups.insert(instance_context(group)) {
                    continue;
                }
                collect(group.members(), classes, seen);
                if matches!(group.declared_in(), MemberContext::Component(_)) {
                    classes.push(Shared::Group(group));
                }
            }
            dictionary::Member::Component(component) => {
                if !seen.components.insert(component.name()) {
                    continue;
                }
                collect(component.members(), classes, seen);
                classes.push(Shared::Component(component));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Shared, shared_classes};
    use crate::test_utils::instrument_dictionary;

    #[test]
    fn a_class_follows_the_classes_it_names() {
        let dictionary = instrument_dictionary();
        let names: Vec<&str> = shared_classes(&dictionary)
            .iter()
            .map(|class| match class {
                Shared::Group(group) => group.component().unwrap(),
                Shared::Component(component) => component.name(),
            })
            .collect();

        assert_eq!(
            names,
            ["SecAltIDGrp", "SecurityXML", "Instrument", "OrderQtyData"]
        );
    }
}
