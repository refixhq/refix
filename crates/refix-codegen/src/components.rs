use std::collections::HashSet;

use refix_dictionary::{Dictionary, Tag, dictionary};

use crate::flat;
use crate::groups::known_tags;

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

/// Whether a component holds a group, itself or through another component.
pub(crate) fn holds_group(component: dictionary::Component<'_>) -> bool {
    flat::members(component.members()).any(|member| matches!(member, flat::Member::Group(_)))
}

/// Every tag a component can contain at any depth, sorted.
///
/// Group count tags are included.
pub(crate) fn component_tags(component: dictionary::Component<'_>) -> Vec<Tag> {
    known_tags(flat::members(component.members()))
}

#[cfg(test)]
mod tests {
    use super::{component_tags, components, holds_group};
    use crate::test_utils::instrument_dictionary;
    use refix_dictionary::Tag;

    #[test]
    fn every_component_with_fields_is_found_once_outermost_first() {
        let dictionary = instrument_dictionary();
        let names: Vec<&str> = components(&dictionary)
            .into_iter()
            .map(|component| component.name())
            .collect();

        assert_eq!(names, ["Instrument", "SecurityXML", "OrderQtyData"]);
    }

    #[test]
    fn a_group_in_a_nested_component_counts_as_held() {
        let dictionary = instrument_dictionary();
        let held: Vec<bool> = components(&dictionary)
            .into_iter()
            .map(holds_group)
            .collect();

        assert_eq!(held, [true, false, false]);
    }

    #[test]
    fn component_tags_reach_every_depth() {
        let dictionary = instrument_dictionary();
        let instrument = components(&dictionary)[0];

        assert_eq!(
            component_tags(instrument),
            [Tag(55), Tag(454), Tag(455), Tag(1186)]
        );
    }
}
