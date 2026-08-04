#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Dictionary {
    pub version: Version,
    pub messages: Vec<Message>,
    pub fields: Vec<Field>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Message {
    pub fields: Vec<FieldRef>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Field;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldRef {
    pub tag: u32,
    pub is_required: bool,
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
