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
    let mut resolver = Resolver::new(spec)?;
    resolver.expand_components()?;

    spec.messages
        .iter()
        .map(|message| resolver.resolve_message(message))
        .collect()
}

/// State shared while resolving one spec: the lookup tables, the memoised
/// component expansions, and the components currently being expanded.
struct Resolver<'a> {
    spec: &'a Spec,
    fields_by_tag: HashMap<Tag, FieldIndex>,
    components_by_name: HashMap<&'a str, ComponentIndex>,
    expansions: Vec<Option<Vec<ResolvedMember>>>,
    stack: Vec<ComponentIndex>,
}

impl<'a> Resolver<'a> {
    fn new(spec: &'a Spec) -> Result<Self, Error> {
        Ok(Self {
            spec,
            fields_by_tag: index_fields(&spec.fields)?,
            components_by_name: index_components(&spec.components)?,
            expansions: vec![None; spec.components.len()],
            stack: Vec::new(),
        })
    }

    /// Expands every component definition into its flat member list, with
    /// requiredness relative to the component itself.
    fn expand_components(&mut self) -> Result<(), Error> {
        for index in (0..self.spec.components.len()).map(ComponentIndex) {
            self.expand_component(index)?;
        }
        Ok(())
    }

    fn expand_component(&mut self, index: ComponentIndex) -> Result<(), Error> {
        if self.expansions[index.0].is_some() {
            return Ok(());
        }
        let spec = self.spec;
        let component = &spec.components.as_slice()[index];
        if self.stack.contains(&index) {
            return Err(Error::CircularComponent {
                component: component.name.clone(),
            });
        }

        let context = MemberContext::Component(component.name.clone());
        self.stack.push(index);
        let members = self.resolve_members(&component.members, &context)?;
        self.stack.pop();

        self.expansions[index.0] = Some(members);

        Ok(())
    }

    fn resolve_message(&mut self, message: &spec::Message) -> Result<ResolvedMessage, Error> {
        let context = MemberContext::Message(message.name.clone());
        let members = self.resolve_members(&message.members, &context)?;
        Ok(ResolvedMessage { members })
    }

    /// Resolves one member list, expanding component references in
    /// place, and checks that no tag appears in it twice.
    fn resolve_members(
        &mut self,
        members: &[spec::Member],
        context: &MemberContext,
    ) -> Result<Vec<ResolvedMember>, Error> {
        let mut resolved = Vec::new();
        for member in members {
            match member {
                spec::Member::Field(field_ref) => {
                    resolved.push(self.resolve_field_ref(field_ref, context)?);
                }
                spec::Member::Component(component_ref) => {
                    let Some(&child) = self.components_by_name.get(component_ref.name.as_str())
                    else {
                        return Err(Error::UnknownComponent {
                            context: context.clone(),
                            component: component_ref.name.clone(),
                        });
                    };
                    self.expand_component(child)?;
                    let expansion = self.expansions[child.0]
                        .as_ref()
                        .expect("the call above expands the child");
                    extend_with_expansion(&mut resolved, expansion, component_ref.is_required);
                }
            }
        }

        check_unique_tags(&resolved, context, &self.spec.fields)?;
        Ok(resolved)
    }

    fn resolve_field_ref(
        &self,
        field_ref: &spec::FieldRef,
        context: &MemberContext,
    ) -> Result<ResolvedMember, Error> {
        let Some(&field_index) = self.fields_by_tag.get(&field_ref.tag) else {
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
