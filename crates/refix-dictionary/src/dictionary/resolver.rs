use super::{
    ComponentIndex, Dictionary, Error, FieldIndex, Resolved, ResolvedComponent, ResolvedGroup,
    ResolvedMember, ResolvedMessage, Warning,
};
use crate::spec::{self, DataType, Field, Spec};
use crate::{MemberContext, Tag};
use std::collections::{HashMap, hash_map::Entry};

impl Spec {
    /// Resolves this spec into a [`Dictionary`], checking its integrity.
    ///
    /// A message or group instance and the components it includes may list
    /// the same field. A tag listed again in any other way is dropped with
    /// a [`Warning`].
    pub fn resolve(self) -> Result<Resolved, Error> {
        let mut resolver = Resolver::new(&self)?;
        resolver.expand_components()?;
        let mut header = resolver.resolve_members(&self.header, &MemberContext::Header, false)?;
        let mut trailer =
            resolver.resolve_members(&self.trailer, &MemberContext::Trailer, false)?;
        let mut fixed = Listings::default();
        resolver.drop_repeated_tags(&mut header, &Scope::of(&MemberContext::Header), &mut fixed)?;
        resolver.drop_repeated_tags(
            &mut trailer,
            &Scope::of(&MemberContext::Trailer),
            &mut fixed,
        )?;
        let messages = self
            .messages
            .iter()
            .map(|message| resolver.resolve_message(message, &fixed))
            .collect::<Result<_, _>>()?;
        let warnings = resolver.warnings;

        Ok(Resolved {
            dictionary: Dictionary {
                spec: self,
                header,
                messages,
                trailer,
            },
            warnings,
        })
    }
}

/// State shared while resolving one spec.
///
/// It holds the lookup tables, the memoised component expansions, the
/// components currently being expanded and the warnings so far.
struct Resolver<'a> {
    spec: &'a Spec,
    fields_by_tag: HashMap<Tag, FieldIndex>,
    components_by_name: HashMap<&'a str, ComponentIndex>,
    expansions: Vec<Option<Vec<ResolvedMember>>>,
    stack: Vec<ComponentIndex>,
    warnings: Vec<Warning>,
}

impl<'a> Resolver<'a> {
    fn new(spec: &'a Spec) -> Result<Self, Error> {
        check_int_values(&spec.fields)?;
        check_unique_codes(&spec.fields)?;
        check_unique_msg_types(&spec.messages)?;
        let fields_by_tag = index_fields(&spec.fields)?;
        check_length_tags(&spec.fields, &fields_by_tag)?;
        Ok(Self {
            spec,
            fields_by_tag,
            components_by_name: index_components(&spec.components)?,
            expansions: vec![None; spec.components.len()],
            stack: Vec::new(),
            warnings: Vec::new(),
        })
    }

    /// Resolves every component definition into its member list, with
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
        let mut members = self.resolve_members(&component.members, &context, false)?;
        self.stack.pop();

        if let [spec::Member::Group(_)] = component.members.as_slice()
            && let [ResolvedMember::Group(group)] = members.as_mut_slice()
        {
            group.component = Some(component.name.clone());
        }

        self.expansions[index.0] = Some(members);

        Ok(())
    }

    /// Resolves a message's members, dropping the tags of the header and
    /// trailer, recorded in `fixed`.
    fn resolve_message(
        &mut self,
        message: &spec::Message,
        fixed: &Listings,
    ) -> Result<ResolvedMessage, Error> {
        let context = MemberContext::Message(message.name.clone());
        let mut members = self.resolve_members(&message.members, &context, false)?;
        self.drop_repeated_tags(&mut members, &Scope::of(&context), &mut fixed.clone())?;
        Ok(ResolvedMessage { members })
    }

    /// Resolves one member list and drops every tag it may not list again.
    ///
    /// A component that carries fields stays one member. The groups of a
    /// component made up of groups alone are spliced in.
    ///
    /// In a group instance a component in first position becomes required,
    /// so that the field every instance starts with is present.
    fn resolve_members(
        &mut self,
        members: &[spec::Member],
        context: &MemberContext,
        is_instance: bool,
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
                    let is_required = component_ref.is_required || (is_instance && position == 0);
                    if expansion.iter().any(ResolvedMember::carries_fields) {
                        resolved.push(ResolvedMember::Component(ResolvedComponent {
                            name: component_ref.name.clone(),
                            is_required,
                            members: expansion.clone(),
                        }));
                    } else {
                        extend_with_expansion(&mut resolved, expansion, is_required);
                    }
                }
                spec::Member::Group(group) => {
                    resolved.push(ResolvedMember::Group(self.resolve_group(group, context)?));
                }
            }
        }

        self.drop_repeated_tags(&mut resolved, &Scope::of(context), &mut Listings::default())?;
        Ok(resolved)
    }

    /// Drops every member whose tag is already listed, in document order
    /// and at any depth, with a warning for each.
    ///
    /// A component shares the scope of whatever includes it. A field stays
    /// when its earlier listings are in the same scope and came through
    /// other components.
    fn drop_repeated_tags(
        &mut self,
        members: &mut Vec<ResolvedMember>,
        scope: &Scope,
        listings: &mut Listings,
    ) -> Result<(), Error> {
        let spec = self.spec;
        let mut kept = Vec::with_capacity(members.len());
        for mut member in members.drain(..) {
            if let ResolvedMember::Component(component) = &mut member {
                let scope = scope.through(&component.name);
                self.drop_repeated_tags(&mut component.members, &scope, listings)?;
                if !component.members.is_empty() {
                    kept.push(member);
                }
                continue;
            }
            let tag = spec.fields.as_slice()[member.first_field()].tag;
            match listings.entry(tag) {
                Entry::Occupied(mut first) => {
                    let first = first.get_mut();
                    if member.carries_fields() && first.may_be_shared_from(scope) {
                        first.paths.push(scope.path.clone());
                        kept.push(member);
                    } else {
                        self.warnings.push(Warning::DuplicateField {
                            tag,
                            kept: first.context.clone(),
                            dropped: scope.context.clone(),
                        });
                    }
                }
                Entry::Vacant(slot) => {
                    slot.insert(Listing::of(&member, scope));
                    if let ResolvedMember::Group(group) = &mut member {
                        let count_field = &spec.fields.as_slice()[group.count_field_index];
                        let instance = scope.context.group(&count_field.name);
                        let scope = Scope::of(&instance);
                        self.drop_repeated_tags(&mut group.members, &scope, listings)?;
                        group.delimiter_index = delimit(&mut group.members, &instance)?;
                    }
                    kept.push(member);
                }
            }
        }
        *members = kept;
        Ok(())
    }

    /// Resolves a group's count field in the enclosing scope, and its
    /// instance members with requiredness relative to the instance.
    fn resolve_group(
        &mut self,
        group: &spec::Group,
        context: &MemberContext,
    ) -> Result<ResolvedGroup, Error> {
        let count_field_index = self.field_index(group.count_tag, context)?;
        let spec = self.spec;
        let instance = context.group(&spec.fields.as_slice()[count_field_index].name);
        let mut members = self.resolve_members(&group.members, &instance, true)?;
        let delimiter_index = delimit(&mut members, &instance)?;

        Ok(ResolvedGroup {
            count_field_index,
            delimiter_index,
            is_required: group.is_required,
            declared_in: context.clone(),
            component: None,
            members,
        })
    }

    fn resolve_field_ref(
        &self,
        field_ref: &spec::FieldRef,
        context: &MemberContext,
    ) -> Result<ResolvedMember, Error> {
        let field_index = self.field_index(field_ref.tag, context)?;
        let field = &self.spec.fields.as_slice()[field_index];
        if field.data_type.has_length_field() && field.length_tag.is_none() {
            return Err(Error::MissingLengthTag {
                field: field.name.clone(),
                context: context.clone(),
            });
        }
        Ok(ResolvedMember::Field {
            field_index,
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

/// The first listing of each tag in a message, at any depth.
type Listings = HashMap<Tag, Listing>;

/// Where a message, section or group instance lists its members.
struct Scope {
    context: MemberContext,
    /// The components the members are listed through, outermost first.
    path: Vec<String>,
}

impl Scope {
    /// The scope's own member list.
    fn of(context: &MemberContext) -> Self {
        Self {
            context: context.clone(),
            path: Vec::new(),
        }
    }

    /// The member list of `component`, included in this one.
    fn through(&self, component: &str) -> Self {
        let mut path = self.path.clone();
        path.push(component.to_owned());
        Self {
            context: self.context.clone(),
            path,
        }
    }
}

/// The scope that lists a tag first.
#[derive(Clone)]
struct Listing {
    context: MemberContext,
    /// The component paths that list the tag as a field.
    ///
    /// A group's count tag has none, so it is never shared.
    paths: Vec<Vec<String>>,
}

impl Listing {
    fn of(member: &ResolvedMember, scope: &Scope) -> Self {
        let paths = if member.carries_fields() {
            vec![scope.path.clone()]
        } else {
            Vec::new()
        };
        Self {
            context: scope.context.clone(),
            paths,
        }
    }

    /// Whether `scope` may list the field as well.
    ///
    /// It may when it is the same scope and reaches the field through
    /// components that have not listed it yet.
    fn may_be_shared_from(&self, scope: &Scope) -> bool {
        self.context == scope.context && !self.paths.is_empty() && !self.paths.contains(&scope.path)
    }
}

impl ResolvedMember {
    /// The field this member starts with on the wire, or a group's count field.
    fn first_field(&self) -> FieldIndex {
        match self {
            ResolvedMember::Field { field_index, .. } => *field_index,
            ResolvedMember::Group(group) => group.count_field_index,
            ResolvedMember::Component(component) => component.members[0].first_field(),
        }
    }

    /// Whether this is a field or a component that carries fields.
    fn carries_fields(&self) -> bool {
        !matches!(self, ResolvedMember::Group(_))
    }

    /// Makes this member required in its scope.
    fn require(&mut self) {
        match self {
            ResolvedMember::Field { is_required, .. } => *is_required = true,
            ResolvedMember::Group(group) => group.is_required = true,
            ResolvedMember::Component(component) => {
                component.is_required = true;
                component.members[0].require();
            }
        }
    }

    /// Keeps this member required only if `is_required` holds as well.
    fn combine_required(&mut self, is_required: bool) {
        match self {
            ResolvedMember::Field {
                is_required: own, ..
            } => *own &= is_required,
            ResolvedMember::Group(group) => group.is_required &= is_required,
            ResolvedMember::Component(component) => component.is_required &= is_required,
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

/// Checks that every value listed for an int-based field is a FIX integer.
fn check_int_values(fields: &[Field]) -> Result<(), Error> {
    let int_fields = fields.iter().filter(|field| field.data_type.is_int_based());
    for field in int_fields {
        if let Some(value) = field.values.iter().find(|value| !is_integer(&value.value)) {
            return Err(Error::NonIntegerValue {
                field: field.name.clone(),
                value: value.value.clone(),
            });
        }
    }
    Ok(())
}

/// Checks that no field lists two values under one code.
fn check_unique_codes(fields: &[Field]) -> Result<(), Error> {
    for field in fields {
        let mut codes = HashMap::new();
        for value in &field.values {
            let code = field.data_type.canonical_code(&value.value);
            if let Some(first) = codes.insert(code, &value.value) {
                return Err(Error::RepeatedCode {
                    field: field.name.clone(),
                    first: first.clone(),
                    second: value.value.clone(),
                });
            }
        }
    }
    Ok(())
}

/// Checks that no two messages declare the same MsgType.
fn check_unique_msg_types(messages: &[spec::Message]) -> Result<(), Error> {
    let mut names = HashMap::new();
    for message in messages {
        if let Some(first) = names.insert(message.msg_type.as_str(), &message.name) {
            return Err(Error::DuplicateMsgType {
                msg_type: message.msg_type.clone(),
                first: first.clone(),
                second: message.name.clone(),
            });
        }
    }
    Ok(())
}

/// Checks that only data fields take a length, each from a LENGTH field of
/// its own.
fn check_length_tags(
    fields: &[Field],
    fields_by_tag: &HashMap<Tag, FieldIndex>,
) -> Result<(), Error> {
    let mut by_length_tag = HashMap::new();
    for field in fields {
        let Some(tag) = field.length_tag else {
            continue;
        };
        if !field.data_type.has_length_field() {
            return Err(Error::UnexpectedLengthTag {
                field: field.name.clone(),
                tag,
            });
        }
        let Some(&index) = fields_by_tag.get(&tag) else {
            return Err(Error::UnknownLengthTag {
                field: field.name.clone(),
                tag,
            });
        };
        let length_field = &fields[index];
        if length_field.data_type != DataType::Length {
            return Err(Error::NonLengthField {
                field: field.name.clone(),
                length_field: length_field.name.clone(),
            });
        }
        if let Some(first) = by_length_tag.insert(tag, &field.name) {
            return Err(Error::SharedLengthField {
                length_field: length_field.name.clone(),
                first: first.clone(),
                second: field.name.clone(),
            });
        }
    }
    Ok(())
}

fn is_integer(value: &str) -> bool {
    let digits = value.strip_prefix('-').unwrap_or(value);
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) && value.parse::<i64>().is_ok()
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
/// requiredness applies to the expansion's own members; the instances of
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

/// Makes the first member of an instance its delimiter, required whatever
/// its declared flag.
fn delimit(members: &mut [ResolvedMember], instance: &MemberContext) -> Result<FieldIndex, Error> {
    let Some(first) = members.first_mut() else {
        return Err(Error::EmptyGroup {
            context: instance.clone(),
        });
    };
    first.require();
    Ok(first.first_field())
}
