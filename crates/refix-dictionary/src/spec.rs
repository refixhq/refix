use refix_message::Tag;
use std::fmt;

/// A FIX data dictionary as authored: the field, message and component
/// definitions of one FIX version or venue dialect.
///
/// A spec is plain data and makes no consistency guarantees. It is what
/// frontends parse into and what dialect authors construct by hand.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Spec {
    pub version: Version,
    pub messages: Vec<Message>,
    pub fields: Vec<Field>,
    pub components: Vec<Component>,
}

/// The definition of a FIX message type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Message {
    /// The name of the message type.
    pub name: String,
    /// The wire value of `MsgType(35)`, verbatim, e.g. `"D"`.
    pub msg_type: String,
    /// The message's members in source order.
    pub members: Vec<Member>,
    /// The category of this message.
    pub category: Category,
}

/// A field definition, as listed in the dictionary's fields section.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Field {
    pub name: String,
    pub tag: Tag,
    pub data_type: DataType,
    pub values: Vec<EnumValue>,
}

/// A component definition, as listed in the dictionary's components section.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Component {
    /// The name of the component.
    pub name: String,
    /// The component's members in source order.
    pub members: Vec<Member>,
}

/// A member of a message or component.
///
/// This can either be a field reference or a component reference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Member {
    Field(FieldRef),
    Component(ComponentRef),
}

/// A field as used by one message or component (Orchestra's `fieldRef`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldRef {
    /// Tag of the referenced [`Field`] definition.
    pub tag: Tag,
    pub is_required: bool,
}

/// A component as used by one message or component.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentRef {
    /// Name of the referenced [`Component`] definition.
    pub name: String,
    pub is_required: bool,
}

/// The owner of a member list: the message or component a member appears in.
///
/// Diagnostics use this to name the place a problem was found.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MemberContext {
    Message(String),
    Component(String),
    /// A group's entries, named after the group's count field.
    Group {
        name: String,
        parent: Box<MemberContext>,
    },
}

impl fmt::Display for MemberContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Message(name) => write!(f, "message '{name}'"),
            Self::Component(name) => write!(f, "component '{name}'"),
            Self::Group { name, parent } => write!(f, "group '{name}' in {parent}"),
        }
    }
}

/// An enum variant's value.
///
/// This models the `<value enum="..." description="..." />` elements in a field definition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumValue {
    pub value: String,
    pub description: String,
}

/// The FIX data type of a field, as declared in the dictionary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataType {
    String,
    Int,
    /// A type no consumer interprets yet, e.g. `"PRICE"`.
    Other(String),
}

/// The FIX version a dictionary describes, e.g. FIX 4.4 or FIXT 1.1.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Version {
    pub protocol: Protocol,
    pub major: u8,
    pub minor: u8,
    pub service_pack: u8,
}

/// The protocol family a dictionary belongs to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Protocol {
    /// Classic FIX (4.x and earlier), session and application layers in
    /// one dictionary.
    Fix,
    /// The FIXT transport (FIX 5.x onwards), where the session layer is
    /// versioned separately from application semantics.
    Fixt,
}

/// Whether a message belongs to the session layer or the application layer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Category {
    /// Session-level machinery: Logon, Heartbeat, Reject, etc.
    Admin,
    /// Business messages: orders, executions, market data.
    App,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_nested_group_context_names_its_whole_path() {
        let context = MemberContext::Group {
            name: "NoPartySubIDs".to_owned(),
            parent: Box::new(MemberContext::Group {
                name: "NoPartyIDs".to_owned(),
                parent: Box::new(MemberContext::Component("Parties".to_owned())),
            }),
        };

        assert_eq!(
            context.to_string(),
            "group 'NoPartySubIDs' in group 'NoPartyIDs' in component 'Parties'"
        );

        assert_eq!(
            context.to_string(),
            "group 'NoPartySubIDs' in group 'NoPartyIDs' in component 'Parties'"
        );
    }
}
