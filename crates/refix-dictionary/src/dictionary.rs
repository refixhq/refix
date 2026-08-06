/// A FIX data dictionary.
///
/// This contains the field and message definitions of one FIX version
/// or venue dialect.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Dictionary {
    pub version: Version,
    pub messages: Vec<Message>,
    pub fields: Vec<Field>,
}

/// The definition of a FIX message type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Message {
    /// The name of the message type.
    pub name: String,
    /// The wire value of `MsgType(35)`, verbatim, e.g. `"D"`.
    pub msg_type: String,
    /// The message's fields in source order.
    pub fields: Vec<FieldRef>,
    /// The category of this message.
    pub category: Category,
}

/// A field definition, as listed in the dictionary's fields section.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Field {
    pub name: String,
    pub tag: u32,
    pub data_type: DataType,
    pub values: Vec<EnumValue>,
}

/// A field as used by one message (Orchestra's `fieldRef`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldRef {
    /// Tag of the referenced [`Field`] definition.
    pub tag: u32,
    pub is_required: bool,
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
