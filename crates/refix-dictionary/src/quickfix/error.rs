use crate::{MemberContext, dictionary, transport};
use std::fmt;

/// A defect that prevents producing a dictionary at all.
#[derive(Debug)]
pub enum Error {
    Xml(roxmltree::Error),
    UnexpectedRoot(String),
    UnknownProtocol(String),
    MissingAttribute {
        element: String,
        attribute: String,
    },
    InvalidNumber {
        element: String,
        attribute: String,
        value: String,
    },
    UnknownField {
        context: MemberContext,
        field: String,
    },
    DuplicateField {
        field: String,
    },
    InvalidAttribute {
        element: String,
        attribute: String,
        value: String,
    },
    /// A data field listed after different LENGTH fields.
    ConflictingLengthFields {
        field: String,
        first: String,
        second: String,
    },
    Invalid(dictionary::Error),
    /// A defect in the transport dictionary.
    Transport(Box<Error>),
    /// An application dictionary and a transport dictionary that cannot be
    /// joined.
    Unjoinable(transport::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Xml(error) => write!(f, "malformed xml: {error}"),
            Error::UnexpectedRoot(root) => {
                write!(f, "unexpected root element <{root}>, expected <fix>")
            }
            Error::UnknownProtocol(protocol) => {
                write!(
                    f,
                    "unknown protocol type '{protocol}', expected FIX or FIXT"
                )
            }
            Error::MissingAttribute { element, attribute } => {
                write!(f, "missing attribute '{attribute}' on <{element}>")
            }
            Error::InvalidNumber {
                element,
                attribute,
                value,
            } => {
                write!(
                    f,
                    "invalid number '{value}' in attribute '{attribute}' of <{element}>"
                )
            }
            Error::UnknownField { context, field } => {
                write!(f, "{context} references unknown field '{field}'")
            }
            Error::DuplicateField { field } => {
                write!(f, "field '{field}' is defined more than once")
            }
            Error::InvalidAttribute {
                element,
                attribute,
                value,
            } => {
                write!(
                    f,
                    "invalid value '{value}' in attribute '{attribute}' of <{element}>"
                )
            }
            Error::ConflictingLengthFields {
                field,
                first,
                second,
            } => {
                write!(
                    f,
                    "field '{field}' takes its length from '{first}' in one place and '{second}' in another"
                )
            }
            Error::Invalid(error) => {
                write!(f, "invalid dictionary: {error}")
            }
            Error::Transport(error) => write!(f, "in the transport dictionary: {error}"),
            Error::Unjoinable(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Xml(error) => Some(error),
            Error::Invalid(error) => Some(error),
            Error::Transport(error) => Some(error),
            Error::Unjoinable(error) => Some(error),
            _ => None,
        }
    }
}
