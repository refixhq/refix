//! The validated, resolved form of a data dictionary.
//!
//! A [`Dictionary`] can only be obtained by resolving a [`Spec`], so
//! holding one proves its references are valid, and reads are infallible.

mod error;
mod resolver;

use crate::{Category, Field, Spec, Version, spec};
pub use error::Error;
use std::fmt;
use std::ops::Index;

/// A resolved data dictionary, produced by [`Spec::resolve`].
#[derive(Clone, Debug)]
pub struct Dictionary {
    spec: Spec,
    messages: Vec<ResolvedMessage>,
}

#[derive(Clone, Debug)]
struct ResolvedMessage {
    members: Vec<ResolvedMember>,
}

/// A member as indices into the spec's field definitions, so the
/// dictionary stays free of self-references.
#[derive(Clone, Debug, Eq, PartialEq)]
enum ResolvedMember {
    Field {
        field_index: FieldIndex,
        is_required: bool,
    },
    Group(ResolvedGroup),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ResolvedGroup {
    count_field_index: FieldIndex,
    delimiter_index: FieldIndex,
    is_required: bool,
    members: Vec<ResolvedMember>,
}

/// A position in the spec's field definitions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FieldIndex(usize);

/// A position in the spec's component definitions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ComponentIndex(usize);

impl Index<FieldIndex> for [Field] {
    type Output = Field;

    fn index(&self, index: FieldIndex) -> &Field {
        &self[index.0]
    }
}

impl Index<ComponentIndex> for [spec::Component] {
    type Output = spec::Component;

    fn index(&self, index: ComponentIndex) -> &spec::Component {
        &self[index.0]
    }
}

impl Dictionary {
    pub fn version(&self) -> Version {
        self.spec.version
    }

    /// The field definitions, as declared in the spec.
    pub fn fields(&self) -> &[Field] {
        &self.spec.fields
    }

    /// The messages, with references resolved and components expanded.
    pub fn messages(&self) -> impl Iterator<Item = Message<'_>> {
        self.spec
            .messages
            .iter()
            .zip(&self.messages)
            .map(|(declared, resolved)| Message {
                declared,
                members: &resolved.members,
                fields: &self.spec.fields,
            })
    }

    /// The spec this dictionary was resolved from, for structural queries.
    pub fn spec(&self) -> &Spec {
        &self.spec
    }
}

/// A read view of one message, with its member list resolved.
#[derive(Clone, Copy)]
pub struct Message<'a> {
    declared: &'a spec::Message,
    members: &'a [ResolvedMember],
    fields: &'a [Field],
}

impl<'a> Message<'a> {
    pub fn name(&self) -> &'a str {
        &self.declared.name
    }

    /// The wire value of `MsgType(35)`, verbatim, e.g. `"D"`.
    pub fn msg_type(&self) -> &'a str {
        &self.declared.msg_type
    }

    pub fn category(&self) -> Category {
        self.declared.category
    }

    /// The members in source order, components expanded in place.
    pub fn members(&self) -> impl Iterator<Item = Member<'a>> {
        members_of(self.members, self.fields)
    }
}

/// A member of a message, with its references resolved.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Member<'a> {
    Field {
        /// The referenced field definition.
        field: &'a Field,
        /// Whether the field is required, combined across every component
        /// on the path to it: required only if required at every level.
        is_required: bool,
    },
    Group(Group<'a>),
}

/// A read view of one repeating group, with its entry members resolved.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Group<'a> {
    resolved: &'a ResolvedGroup,
    fields: &'a [Field],
}

impl<'a> Group<'a> {
    /// The NumInGroup field, e.g. `NoPartyIDs(453)`.
    pub fn count_field(&self) -> &'a Field {
        &self.fields[self.resolved.count_field_index]
    }

    /// The field every entry starts with: the entry's first field after
    /// expansion, or a nested group's count field.
    pub fn delimiter(&self) -> &'a Field {
        &self.fields[self.resolved.delimiter_index]
    }

    /// Whether the count field is required in the enclosing scope,
    /// combined across every component on the path to it.
    pub fn is_required(&self) -> bool {
        self.resolved.is_required
    }

    /// The members of each entry in source order, with requiredness
    /// relative to the entry.
    pub fn members(&self) -> impl Iterator<Item = Member<'a>> {
        members_of(&self.resolved.members, self.fields)
    }
}

impl fmt::Debug for Group<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Group")
            .field("count_field", &self.count_field().name)
            .field("is_required", &self.is_required())
            .field("members", &self.members().collect::<Vec<_>>())
            .finish()
    }
}

fn members_of<'a>(
    members: &'a [ResolvedMember],
    fields: &'a [Field],
) -> impl Iterator<Item = Member<'a>> {
    members.iter().map(move |member| match member {
        ResolvedMember::Field {
            field_index,
            is_required,
        } => Member::Field {
            field: &fields[*field_index],
            is_required: *is_required,
        },
        ResolvedMember::Group(group) => Member::Group(Group {
            resolved: group,
            fields,
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Component, ComponentRef, DataType, FieldRef, MemberContext, Protocol, Tag};

    fn field(name: &str, tag: u32) -> Field {
        Field {
            name: name.to_owned(),
            tag: Tag(tag),
            data_type: DataType::String,
            values: vec![],
        }
    }

    fn field_ref(tag: u32, is_required: bool) -> spec::Member {
        spec::Member::Field(FieldRef {
            tag: Tag(tag),
            is_required,
        })
    }

    /// Members as (tag, is_required), for member lists without groups.
    fn field_members<'a>(members: impl Iterator<Item = Member<'a>>) -> Vec<(Tag, bool)> {
        members
            .map(|member| match member {
                Member::Field { field, is_required } => (field.tag, is_required),
                Member::Group(group) => panic!("unexpected group {}", group.count_field().name),
            })
            .collect()
    }

    fn component_ref(name: &str, is_required: bool) -> spec::Member {
        spec::Member::Component(ComponentRef {
            name: name.to_owned(),
            is_required,
        })
    }

    fn component(name: &str, members: Vec<spec::Member>) -> Component {
        Component {
            name: name.to_owned(),
            members,
        }
    }

    fn message(name: &str, members: Vec<spec::Member>) -> spec::Message {
        spec::Message {
            name: name.to_owned(),
            msg_type: "D".to_owned(),
            members,
            category: Category::App,
        }
    }

    fn spec_of(
        fields: Vec<Field>,
        components: Vec<Component>,
        messages: Vec<spec::Message>,
    ) -> Spec {
        Spec {
            version: Version {
                protocol: Protocol::Fix,
                major: 4,
                minor: 4,
                service_pack: 0,
            },
            messages,
            fields,
            components,
        }
    }

    #[test]
    fn resolves_direct_field_references() {
        let spec = spec_of(
            vec![field("ClOrdID", 11), field("Text", 58)],
            vec![],
            vec![message(
                "NewOrderSingle",
                vec![field_ref(11, true), field_ref(58, false)],
            )],
        );

        let dictionary = spec.resolve().unwrap();

        let messages: Vec<Message> = dictionary.messages().collect();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].name(), "NewOrderSingle");
        assert_eq!(messages[0].msg_type(), "D");
        assert_eq!(messages[0].category(), Category::App);
        assert_eq!(
            messages[0].members().collect::<Vec<Member>>(),
            vec![
                Member::Field {
                    field: &dictionary.fields()[0],
                    is_required: true,
                },
                Member::Field {
                    field: &dictionary.fields()[1],
                    is_required: false,
                },
            ]
        );
    }

    #[test]
    fn exposes_the_spec_it_was_resolved_from() {
        let spec = spec_of(vec![field("ClOrdID", 11)], vec![], vec![]);

        let dictionary = spec.clone().resolve().unwrap();

        assert_eq!(dictionary.spec(), &spec);
        assert_eq!(dictionary.version(), spec.version);
        assert_eq!(dictionary.fields(), spec.fields.as_slice());
    }

    #[test]
    fn expands_components_in_place() {
        let spec = spec_of(
            vec![
                field("ClOrdID", 11),
                field("Commission", 12),
                field("CommType", 13),
                field("Text", 58),
            ],
            vec![component(
                "CommissionData",
                vec![field_ref(12, true), field_ref(13, true)],
            )],
            vec![message(
                "NewOrderSingle",
                vec![
                    field_ref(11, true),
                    component_ref("CommissionData", true),
                    field_ref(58, false),
                ],
            )],
        );

        let dictionary = spec.resolve().unwrap();

        let message = dictionary.messages().next().unwrap();
        assert_eq!(
            field_members(message.members()),
            vec![
                (Tag(11), true),
                (Tag(12), true),
                (Tag(13), true),
                (Tag(58), false)
            ]
        );
    }

    #[test]
    fn requiredness_combines_across_every_level() {
        let spec = spec_of(
            vec![field("Commission", 12), field("CommType", 13)],
            vec![
                component("Inner", vec![field_ref(12, true), field_ref(13, false)]),
                component("Outer", vec![component_ref("Inner", true)]),
            ],
            vec![message(
                "NewOrderSingle",
                vec![component_ref("Outer", false)],
            )],
        );

        let dictionary = spec.resolve().unwrap();

        // An optional component makes everything below it optional.
        let message = dictionary.messages().next().unwrap();
        assert_eq!(
            field_members(message.members()),
            vec![(Tag(12), false), (Tag(13), false)]
        );
    }

    #[test]
    fn a_required_path_stays_required() {
        let spec = spec_of(
            vec![field("Commission", 12)],
            vec![
                component("Inner", vec![field_ref(12, true)]),
                component("Outer", vec![component_ref("Inner", true)]),
            ],
            vec![message(
                "NewOrderSingle",
                vec![component_ref("Outer", true)],
            )],
        );

        let dictionary = spec.resolve().unwrap();

        let message = dictionary.messages().next().unwrap();
        assert_eq!(field_members(message.members()), vec![(Tag(12), true)]);
    }

    #[test]
    fn an_unknown_component_in_a_message_is_an_error() {
        let spec = spec_of(
            vec![],
            vec![],
            vec![message(
                "NewOrderSingle",
                vec![component_ref("Parties", false)],
            )],
        );

        assert_eq!(
            spec.resolve().unwrap_err(),
            Error::UnknownComponent {
                context: MemberContext::Message("NewOrderSingle".to_owned()),
                component: "Parties".to_owned(),
            }
        );
    }

    #[test]
    fn an_unknown_component_in_a_component_is_an_error() {
        let spec = spec_of(
            vec![],
            vec![component("Outer", vec![component_ref("Inner", false)])],
            vec![],
        );

        assert_eq!(
            spec.resolve().unwrap_err(),
            Error::UnknownComponent {
                context: MemberContext::Component("Outer".to_owned()),
                component: "Inner".to_owned(),
            }
        );
    }

    #[test]
    fn an_unknown_tag_is_an_error() {
        let spec = spec_of(
            vec![],
            vec![],
            vec![message("Heartbeat", vec![field_ref(112, false)])],
        );

        assert_eq!(
            spec.resolve().unwrap_err(),
            Error::UnknownField {
                context: MemberContext::Message("Heartbeat".to_owned()),
                tag: Tag(112),
            }
        );
    }

    #[test]
    fn a_self_referencing_component_is_an_error() {
        let spec = spec_of(
            vec![],
            vec![component("Parties", vec![component_ref("Parties", false)])],
            vec![],
        );

        assert_eq!(
            spec.resolve().unwrap_err(),
            Error::CircularComponent {
                component: "Parties".to_owned(),
            }
        );
    }

    #[test]
    fn mutually_referencing_components_are_an_error() {
        let spec = spec_of(
            vec![],
            vec![
                component(
                    "Instrument",
                    vec![component_ref("UnderlyingInstrument", false)],
                ),
                component(
                    "UnderlyingInstrument",
                    vec![component_ref("Instrument", false)],
                ),
            ],
            vec![],
        );

        assert!(matches!(
            spec.resolve().unwrap_err(),
            Error::CircularComponent { .. }
        ));
    }

    #[test]
    fn an_unused_component_is_still_checked() {
        // No message references the cycle; resolution must find it anyway.
        let spec = spec_of(
            vec![],
            vec![component("Parties", vec![component_ref("Parties", false)])],
            vec![message("Heartbeat", vec![])],
        );

        assert!(matches!(
            spec.resolve().unwrap_err(),
            Error::CircularComponent { .. }
        ));
    }

    #[test]
    fn a_diamond_of_empty_components_is_not_a_cycle() {
        let spec = spec_of(
            vec![],
            vec![
                component(
                    "Top",
                    vec![component_ref("Left", false), component_ref("Right", false)],
                ),
                component("Left", vec![component_ref("Bottom", false)]),
                component("Right", vec![component_ref("Bottom", false)]),
                component("Bottom", vec![]),
            ],
            vec![],
        );

        assert!(spec.resolve().is_ok());
    }

    #[test]
    fn a_diamond_over_a_field_duplicates_the_tag() {
        // Both arms of the diamond contribute tag 58 to Top's expansion.
        let spec = spec_of(
            vec![field("Text", 58)],
            vec![
                component(
                    "Top",
                    vec![component_ref("Left", false), component_ref("Right", false)],
                ),
                component("Left", vec![component_ref("Bottom", false)]),
                component("Right", vec![component_ref("Bottom", false)]),
                component("Bottom", vec![field_ref(58, false)]),
            ],
            vec![],
        );

        assert_eq!(
            spec.resolve().unwrap_err(),
            Error::DuplicateField {
                context: MemberContext::Component("Top".to_owned()),
                tag: Tag(58),
            }
        );
    }

    #[test]
    fn a_duplicate_tag_after_expansion_is_an_error() {
        let spec = spec_of(
            vec![field("Text", 58)],
            vec![component("Notes", vec![field_ref(58, false)])],
            vec![message(
                "NewOrderSingle",
                vec![field_ref(58, true), component_ref("Notes", false)],
            )],
        );

        assert_eq!(
            spec.resolve().unwrap_err(),
            Error::DuplicateField {
                context: MemberContext::Message("NewOrderSingle".to_owned()),
                tag: Tag(58),
            }
        );
    }

    #[test]
    fn duplicate_component_definitions_are_an_error() {
        let spec = spec_of(
            vec![],
            vec![component("Parties", vec![]), component("Parties", vec![])],
            vec![],
        );

        assert_eq!(
            spec.resolve().unwrap_err(),
            Error::DuplicateComponent {
                component: "Parties".to_owned(),
            }
        );
    }

    #[test]
    fn duplicate_field_tags_are_an_error() {
        let spec = spec_of(
            vec![field("ClOrdID", 11), field("OrigClOrdID", 11)],
            vec![],
            vec![],
        );

        assert_eq!(
            spec.resolve().unwrap_err(),
            Error::DuplicateTag { tag: Tag(11) }
        );
    }
}
