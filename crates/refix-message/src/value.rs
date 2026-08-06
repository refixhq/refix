/// A field value whose bytes can't be read as the field's declared data type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidValue {
    pub tag: u32,
}

impl std::fmt::Display for InvalidValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid value in field {}", self.tag)
    }
}

impl std::error::Error for InvalidValue {}
