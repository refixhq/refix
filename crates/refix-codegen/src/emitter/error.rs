#[derive(Debug, Eq, PartialEq)]
pub enum Error {
    UnrepresentableName { field: String },
    UnrepresentableValue { field: String, description: String },
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
        }
    }
}

impl std::error::Error for Error {}
