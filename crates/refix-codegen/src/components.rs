use std::collections::HashSet;

use refix_dictionary::{Dictionary, dictionary};

/// Every component with fields that the header, messages and trailer reach.
///
/// Every component appears once, in order of first appearance.
/// Each is the first use the dictionary lists.
pub(crate) fn components(dictionary: &Dictionary) -> Vec<dictionary::Component<'_>> {
    let mut components = Vec::new();
    let mut seen = HashSet::new();
    collect(dictionary.header(), &mut components, &mut seen);
    for message in dictionary.messages() {
        collect(message.members(), &mut components, &mut seen);
    }
    collect(dictionary.trailer(), &mut components, &mut seen);
    components
}

fn collect<'a>(
    members: impl Iterator<Item = dictionary::Member<'a>>,
    components: &mut Vec<dictionary::Component<'a>>,
    seen: &mut HashSet<&'a str>,
) {
    for member in members {
        match member {
            dictionary::Member::Field { .. } => {}
            dictionary::Member::Group(group) => collect(group.members(), components, seen),
            dictionary::Member::Component(component) => {
                if seen.insert(component.name()) {
                    components.push(component);
                    collect(component.members(), components, seen);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::components;
    use crate::test_utils::instrument_dictionary;

    #[test]
    fn every_component_with_fields_is_found_once_outermost_first() {
        let dictionary = instrument_dictionary();
        let names: Vec<&str> = components(&dictionary)
            .into_iter()
            .map(|component| component.name())
            .collect();

        assert_eq!(names, ["Instrument", "SecurityXML", "OrderQtyData"]);
    }
}
