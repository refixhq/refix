//! The validated, resolved form of a data dictionary.
//!
//! A [`Dictionary`] can only be obtained by resolving a [`Spec`], so
//! holding one proves its references are valid, and reads are infallible.

mod error;

use crate::{Category, Field, MemberContext, Spec, Tag, Version, spec};
pub use error::Error;
use std::collections::{HashMap, HashSet};
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

/// A member as an index into the spec's field definitions, so the
/// dictionary stays free of self-references.
#[derive(Clone, Copy, Debug)]
struct ResolvedMember {
    field_index: FieldIndex,
    is_required: bool,
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

impl Spec {
    /// Resolves this spec into a [`Dictionary`], checking its integrity.
    pub fn resolve(self) -> Result<Dictionary, Error> {
        let messages = resolve_messages(&self)?;

        Ok(Dictionary {
            spec: self,
            messages,
        })
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
        let fields = self.fields;
        self.members.iter().map(move |member| Member {
            field: &fields[member.field_index],
            is_required: member.is_required,
        })
    }
}

/// A field as used by one message, with its reference resolved.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Member<'a> {
    /// The referenced field definition.
    pub field: &'a Field,
    /// Whether the field is required, combined across every component
    /// on the path to it: required only if required at every level.
    pub is_required: bool,
}

fn resolve_messages(spec: &Spec) -> Result<Vec<ResolvedMessage>, Error> {
    let fields_by_tag = index_fields(&spec.fields)?;
    let components_by_name = index_components(&spec.components)?;
    let expansions = expand_components(spec, &components_by_name, &fields_by_tag)?;

    spec.messages
        .iter()
        .map(|message| {
            resolve_message(
                message,
                spec,
                &components_by_name,
                &expansions,
                &fields_by_tag,
            )
        })
        .collect()
}

fn index_fields(fields: &[Field]) -> Result<HashMap<Tag, FieldIndex>, Error> {
    let mut by_tag = HashMap::new();
    for (index, field) in fields.iter().enumerate() {
        if by_tag.insert(field.tag, FieldIndex(index)).is_some() {
            return Err(Error::DuplicateTag { tag: field.tag });
        }
    }
    Ok(by_tag)
}

fn index_components(
    components: &[spec::Component],
) -> Result<HashMap<&str, ComponentIndex>, Error> {
    let mut by_name = HashMap::new();
    for (index, component) in components.iter().enumerate() {
        if by_name
            .insert(component.name.as_str(), ComponentIndex(index))
            .is_some()
        {
            return Err(Error::DuplicateComponent {
                component: component.name.clone(),
            });
        }
    }
    Ok(by_name)
}

/// Expands every component definition into its flat member list, with
/// requiredness relative to the component itself.
fn expand_components(
    spec: &Spec,
    by_name: &HashMap<&str, ComponentIndex>,
    fields_by_tag: &HashMap<Tag, FieldIndex>,
) -> Result<Vec<Vec<ResolvedMember>>, Error> {
    let mut expansions = vec![None; spec.components.len()];
    for index in (0..spec.components.len()).map(ComponentIndex) {
        expand_component(
            index,
            spec,
            by_name,
            fields_by_tag,
            &mut Vec::new(),
            &mut expansions,
        )?;
    }
    Ok(expansions
        .into_iter()
        .map(|expansion| expansion.expect("the loop above expands every component"))
        .collect())
}

fn expand_component(
    index: ComponentIndex,
    spec: &Spec,
    by_name: &HashMap<&str, ComponentIndex>,
    fields_by_tag: &HashMap<Tag, FieldIndex>,
    stack: &mut Vec<ComponentIndex>,
    expansions: &mut Vec<Option<Vec<ResolvedMember>>>,
) -> Result<(), Error> {
    if expansions[index.0].is_some() {
        return Ok(());
    }
    let component = &spec.components.as_slice()[index];
    if stack.contains(&index) {
        return Err(Error::CircularComponent {
            component: component.name.clone(),
        });
    }

    let context = MemberContext::Component(component.name.clone());
    stack.push(index);
    let mut members = Vec::new();
    for member in &component.members {
        match member {
            spec::Member::Field(field_ref) => {
                members.push(resolve_field_ref(field_ref, &context, fields_by_tag)?);
            }
            spec::Member::Component(component_ref) => {
                let Some(&child) = by_name.get(component_ref.name.as_str()) else {
                    return Err(Error::UnknownComponent {
                        context,
                        component: component_ref.name.clone(),
                    });
                };
                expand_component(child, spec, by_name, fields_by_tag, stack, expansions)?;
                let expansion = expansions[child.0]
                    .as_ref()
                    .expect("the recursive call above expands the child");
                extend_with_expansion(&mut members, expansion, component_ref.is_required);
            }
        }
    }
    stack.pop();

    check_unique_tags(&members, &context, &spec.fields)?;
    expansions[index.0] = Some(members);

    Ok(())
}

fn resolve_message(
    message: &spec::Message,
    spec: &Spec,
    by_name: &HashMap<&str, ComponentIndex>,
    expansions: &[Vec<ResolvedMember>],
    fields_by_tag: &HashMap<Tag, FieldIndex>,
) -> Result<ResolvedMessage, Error> {
    let context = MemberContext::Message(message.name.clone());
    let mut members = Vec::new();
    for member in &message.members {
        match member {
            spec::Member::Field(field_ref) => {
                members.push(resolve_field_ref(field_ref, &context, fields_by_tag)?);
            }
            spec::Member::Component(component_ref) => {
                let Some(&component) = by_name.get(component_ref.name.as_str()) else {
                    return Err(Error::UnknownComponent {
                        context,
                        component: component_ref.name.clone(),
                    });
                };
                extend_with_expansion(
                    &mut members,
                    &expansions[component.0],
                    component_ref.is_required,
                );
            }
        }
    }

    check_unique_tags(&members, &context, &spec.fields)?;

    Ok(ResolvedMessage { members })
}

fn resolve_field_ref(
    field_ref: &spec::FieldRef,
    context: &MemberContext,
    fields_by_tag: &HashMap<Tag, FieldIndex>,
) -> Result<ResolvedMember, Error> {
    let Some(&field_index) = fields_by_tag.get(&field_ref.tag) else {
        return Err(Error::UnknownField {
            context: context.clone(),
            tag: field_ref.tag,
        });
    };

    Ok(ResolvedMember {
        field_index,
        is_required: field_ref.is_required,
    })
}

fn extend_with_expansion(
    members: &mut Vec<ResolvedMember>,
    expansion: &[ResolvedMember],
    is_required: bool,
) {
    members.extend(expansion.iter().map(|member| ResolvedMember {
        field_index: member.field_index,
        is_required: is_required && member.is_required,
    }));
}

fn check_unique_tags(
    members: &[ResolvedMember],
    context: &MemberContext,
    fields: &[Field],
) -> Result<(), Error> {
    let mut seen = HashSet::new();
    for member in members {
        let tag = fields[member.field_index].tag;
        if !seen.insert(tag) {
            return Err(Error::DuplicateField {
                context: context.clone(),
                tag,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Component, ComponentRef, DataType, FieldRef, Protocol};

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
                Member {
                    field: &dictionary.fields()[0],
                    is_required: true,
                },
                Member {
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
        let tags: Vec<Tag> = message.members().map(|member| member.field.tag).collect();
        assert_eq!(tags, vec![Tag(11), Tag(12), Tag(13), Tag(58)]);
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
        let required: Vec<bool> = message.members().map(|member| member.is_required).collect();
        assert_eq!(required, vec![false, false]);
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
        assert!(message.members().next().unwrap().is_required);
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
