use super::{
    ComponentIndex, Dictionary, Error, FieldIndex, ResolvedGroup, ResolvedMember, ResolvedMessage,
};
use crate::{Field, MemberContext, Spec, Tag, spec};
use std::collections::{HashMap, hash_map::Entry};

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
        let members = self.resolve_members(&component.members, &context, false)?;
        self.stack.pop();

        self.expansions[index.0] = Some(members);

        Ok(())
    }

    fn resolve_message(&mut self, message: &spec::Message) -> Result<ResolvedMessage, Error> {
        let context = MemberContext::Message(message.name.clone());
        let members = self.resolve_members(&message.members, &context, false)?;
        Ok(ResolvedMessage { members })
    }

    /// Resolves one member list, expanding component references in
    /// place, and checks that no tag appears in it twice.
    ///
    /// In a group entry the first member is required whatever its declared
    /// flag: a component in first position becomes required, and the field
    /// every entry starts with must be present.
    fn resolve_members(
        &mut self,
        members: &[spec::Member],
        context: &MemberContext,
        is_entry: bool,
    ) -> Result<Vec<ResolvedMember>, Error> {
        let mut resolved = Vec::new();
        for (position, member) in members.iter().enumerate() {
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
                    let is_required = component_ref.is_required || (is_entry && position == 0);
                    extend_with_expansion(&mut resolved, expansion, is_required);
                }
                spec::Member::Group(group) => {
                    resolved.push(ResolvedMember::Group(self.resolve_group(group, context)?));
                }
            }
        }

        if is_entry && let Some(first) = resolved.first_mut() {
            first.require();
        }
        check_unique_tags(&resolved, context, &self.spec.fields)?;
        Ok(resolved)
    }

    /// Resolves a group: its count field in the enclosing scope, and its
    /// entry members with requiredness relative to the entry.
    fn resolve_group(
        &mut self,
        group: &spec::Group,
        context: &MemberContext,
    ) -> Result<ResolvedGroup, Error> {
        let count_field_index = self.field_index(group.count_tag, context)?;
        let spec = self.spec;
        let entry = context.group(&spec.fields.as_slice()[count_field_index].name);
        let members = self.resolve_members(&group.members, &entry, true)?;
        let Some(first) = members.first() else {
            return Err(Error::EmptyGroup { context: entry });
        };
        let delimiter_index = first.first_field();

        Ok(ResolvedGroup {
            count_field_index,
            delimiter_index,
            is_required: group.is_required,
            members,
        })
    }

    fn resolve_field_ref(
        &self,
        field_ref: &spec::FieldRef,
        context: &MemberContext,
    ) -> Result<ResolvedMember, Error> {
        Ok(ResolvedMember::Field {
            field_index: self.field_index(field_ref.tag, context)?,
            is_required: field_ref.is_required,
        })
    }

    /// The definition `tag` refers to, where it is used in `context`.
    fn field_index(&self, tag: Tag, context: &MemberContext) -> Result<FieldIndex, Error> {
        self.fields_by_tag
            .get(&tag)
            .copied()
            .ok_or_else(|| Error::UnknownField {
                context: context.clone(),
                tag,
            })
    }
}

impl ResolvedMember {
    /// The field this member starts with on the wire: the field itself, or
    /// a group's count field.
    fn first_field(&self) -> FieldIndex {
        match self {
            ResolvedMember::Field { field_index, .. } => *field_index,
            ResolvedMember::Group(group) => group.count_field_index,
        }
    }

    /// Makes this member required in its scope.
    fn require(&mut self) {
        match self {
            ResolvedMember::Field { is_required, .. } => *is_required = true,
            ResolvedMember::Group(group) => group.is_required = true,
        }
    }

    /// Keeps this member required only if `is_required` holds as well.
    fn combine_required(&mut self, is_required: bool) {
        match self {
            ResolvedMember::Field {
                is_required: own, ..
            } => *own &= is_required,
            ResolvedMember::Group(group) => group.is_required &= is_required,
        }
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

/// Splices a component's expansion into a scope. The reference's
/// requiredness applies to the expansion's own members; the entries of
/// groups within it keep theirs.
fn extend_with_expansion(
    members: &mut Vec<ResolvedMember>,
    expansion: &[ResolvedMember],
    is_required: bool,
) {
    members.extend(expansion.iter().map(|member| {
        let mut member = member.clone();
        member.combine_required(is_required);
        member
    }));
}

fn check_unique_tags(
    members: &[ResolvedMember],
    context: &MemberContext,
    fields: &[Field],
) -> Result<(), Error> {
    insert_unique_tags(members, context, fields, &mut HashMap::new())
}

/// Records where each tag in `members` appears, descending into group
/// entries: a tag may appear only once in a message, at any depth.
fn insert_unique_tags(
    members: &[ResolvedMember],
    context: &MemberContext,
    fields: &[Field],
    seen: &mut HashMap<Tag, MemberContext>,
) -> Result<(), Error> {
    for member in members {
        let tag = fields[member.first_field()].tag;
        match seen.entry(tag) {
            Entry::Occupied(first) => {
                return Err(Error::DuplicateField {
                    tag,
                    first: first.get().clone(),
                    second: context.clone(),
                });
            }
            Entry::Vacant(slot) => {
                slot.insert(context.clone());
            }
        }
        if let ResolvedMember::Group(group) = member {
            let entry = context.group(&fields[group.count_field_index].name);
            insert_unique_tags(&group.members, &entry, fields, seen)?;
        }
    }
    Ok(())
}
