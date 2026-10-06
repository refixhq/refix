use crate::NameClash;

#[derive(Debug, Eq, PartialEq)]
pub enum Error {
    UnrepresentableValue { field: String, description: String },
    NameClash(Box<NameClash>),
}

impl From<Box<NameClash>> for Error {
    fn from(clash: Box<NameClash>) -> Self {
        Error::NameClash(clash)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::UnrepresentableValue { field, description } => {
                write!(
                    f,
                    "value '{description}' of field '{field}' cannot be a python enum member name"
                )
            }
            Error::NameClash(clash) => write!(f, "python name {clash}"),
        }
    }
}

impl std::error::Error for Error {}
