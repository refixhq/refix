use refix_message::Tag;
use std::fmt;

/// The field, message and component definitions of one FIX version or venue
/// dialect, as authored.
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

/// A member of a message, component or group instance.
///
/// This can be a field reference, a component reference or a repeating group.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Member {
    Field(FieldRef),
    Component(ComponentRef),
    Group(Group),
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

/// A repeating group, declared inline in a message, component or group
/// instance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Group {
    /// Tag of the group's NumInGroup count field.
    pub count_tag: Tag,
    pub is_required: bool,
    /// The members of each instance, in source order.
    pub members: Vec<Member>,
}

/// The message, component or group instance a member appears in.
///
/// Diagnostics use this to name the place a problem was found.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MemberContext {
    Message(String),
    Component(String),
    /// A group's instances, named after the group's count field.
    Group {
        name: String,
        parent: Box<MemberContext>,
    },
}

impl MemberContext {
    /// The context of a group's instances, nested in this context.
    pub fn group(&self, name: &str) -> MemberContext {
        MemberContext::Group {
            name: name.to_owned(),
            parent: Box::new(self.clone()),
        }
    }
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
    Length,
    NumInGroup,
    SeqNum,
    TagNum,
    DayOfMonth,
    Char,
    MultipleCharValue,
    /// Also FIX 4.4's `MultipleValueString`.
    MultipleStringValue,
    /// A type no consumer interprets yet, e.g. `"PRICE"`.
    Other(String),
}

impl DataType {
    /// Whether the type is `Int` or one of its subtypes.
    pub fn is_int_based(&self) -> bool {
        matches!(
            self,
            Self::Int
                | Self::Length
                | Self::NumInGroup
                | Self::SeqNum
                | Self::TagNum
                | Self::DayOfMonth
        )
    }

    /// Whether the type holds space-delimited values, as `ExecInst(18)` does.
    pub fn is_multiple_value(&self) -> bool {
        matches!(self, Self::MultipleCharValue | Self::MultipleStringValue)
    }
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
    fn the_int_family_is_int_based() {
        for data_type in [
            DataType::Int,
            DataType::Length,
            DataType::NumInGroup,
            DataType::SeqNum,
            DataType::TagNum,
            DataType::DayOfMonth,
        ] {
            assert!(data_type.is_int_based(), "{data_type:?}");
        }
    }

    #[test]
    fn other_types_are_not_int_based() {
        for data_type in [
            DataType::String,
            DataType::Char,
            DataType::MultipleStringValue,
            DataType::Other("PRICE".to_owned()),
        ] {
            assert!(!data_type.is_int_based(), "{data_type:?}");
        }
    }

    #[test]
    fn the_multiple_value_types_hold_multiple_values() {
        assert!(DataType::MultipleCharValue.is_multiple_value());
        assert!(DataType::MultipleStringValue.is_multiple_value());
        assert!(!DataType::String.is_multiple_value());
        assert!(!DataType::Char.is_multiple_value());
    }

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
