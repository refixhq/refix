#[derive(Debug, Eq, PartialEq)]
pub enum Error {
    UnrepresentableValue { field: String, description: String },
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
        }
    }
}

impl std::error::Error for Error {}
