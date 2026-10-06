use refix_dictionary::MemberContext;

use crate::NameClash;

#[derive(Debug, Eq, PartialEq)]
pub enum Error {
    UnrepresentableName { field: String },
    UnrepresentableValue { field: String, description: String },
    UnrepresentableGroupName { context: MemberContext },
    UnrepresentableMessageName { message: String },
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
            Error::UnrepresentableName { field } => {
                write!(f, "field '{field}' cannot be a rust method name")
            }
            Error::UnrepresentableValue { field, description } => {
                write!(
                    f,
                    "value '{description}' of field '{field}' cannot be a rust variant name"
                )
            }
            Error::UnrepresentableGroupName { context } => {
                write!(f, "{context} cannot be a rust name")
            }
            Error::UnrepresentableMessageName { message } => {
                write!(f, "message '{message}' cannot be a rust module name")
            }
            Error::NameClash(clash) => write!(f, "rust name {clash}"),
        }
    }
}

impl std::error::Error for Error {}
