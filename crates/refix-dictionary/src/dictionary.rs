#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Dictionary {
    pub version: Version,
    pub messages: Vec<Message>,
    pub fields: Vec<Field>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Message {
    pub name: String,
    pub msg_type: String,
    pub fields: Vec<FieldRef>,
    pub category: Category,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Field {
    pub name: String,
    pub tag: u32,
    pub data_type: DataType,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldRef {
    pub tag: u32,
    pub is_required: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataType {
    String,
    Int,
    Other(String), // "PRICE", "UTCTIMESTAMP", etc - not yet interpreted
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Version {
    pub protocol: Protocol,
    pub major: u8,
    pub minor: u8,
    pub service_pack: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Protocol {
    Fix,
    Fixt,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Category {
    Admin,
    App,
}
