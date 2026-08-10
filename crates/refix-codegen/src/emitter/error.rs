#[derive(Debug, Eq, PartialEq)]
pub enum Error {
    UnknownTag { message: String, tag: u32 },
    UnrepresentableName { field: String },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::UnknownTag { message, tag } => {
                write!(f, "message '{message}' references unknown tag {tag}")
            }
            Error::UnrepresentableName { field } => {
                write!(f, "field '{field}' cannot be a rust method name")
            }
        }
    }
}

impl std::error::Error for Error {}
