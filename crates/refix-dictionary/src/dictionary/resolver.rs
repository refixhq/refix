use super::{ComponentIndex, Dictionary, Error, FieldIndex, ResolvedMember, ResolvedMessage};
use crate::{Field, MemberContext, Spec, Tag, spec};
use std::collections::{HashMap, HashSet};

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
