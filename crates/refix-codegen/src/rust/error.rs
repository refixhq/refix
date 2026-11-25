use refix_dictionary::MemberContext;

use crate::NameClash;

#[derive(Debug, Eq, PartialEq)]
pub enum Error {
    UnrepresentableName { field: String },
    UnrepresentableValue { field: String, description: String },
    UnrepresentableGroupName { context: MemberContext },
    UnrepresentableComponentName { component: String },
    NameClash(Box<NameClash>),
    ModuleClash(Box<NameClash>),
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
            Error::UnrepresentableComponentName { component } => {
                write!(f, "component '{component}' cannot be a rust method name")
            }
            Error::NameClash(clash) => write!(f, "rust name {clash}"),
            Error::ModuleClash(clash) => write!(f, "rust module {clash}"),
        }
    }
}

impl std::error::Error for Error {}
