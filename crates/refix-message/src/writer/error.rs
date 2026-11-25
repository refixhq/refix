use crate::Tag;
use std::fmt;

/// A value that cannot be written as a field.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WriteError {
    /// The value is empty.
    EmptyValue { tag: Tag },
    /// The value holds the field separator.
    SeparatorInValue { tag: Tag },
}

impl fmt::Display for WriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WriteError::EmptyValue { tag } => write!(f, "empty value for field {tag}"),
            WriteError::SeparatorInValue { tag } => {
                write!(f, "separator in the value for field {tag}")
            }
        }
    }
}

impl std::error::Error for WriteError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn displays_each_error() {
        let cases = [
            (
                WriteError::EmptyValue { tag: Tag(58) },
                "empty value for field 58",
            ),
            (
                WriteError::SeparatorInValue { tag: Tag(58) },
                "separator in the value for field 58",
            ),
        ];
        for (error, expected) in cases {
            assert_eq!(error.to_string(), expected);
        }
    }
}
