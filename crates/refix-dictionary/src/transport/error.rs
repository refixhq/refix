use crate::Tag;
use std::fmt;

/// Why an application dictionary and a transport dictionary cannot be
/// one dictionary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    /// The application dictionary has members in its own header.
    ApplicationHeader,
    /// The application dictionary has members in its own trailer.
    ApplicationTrailer,
    /// The transport dictionary has no header to give.
    NoTransportHeader,
    /// A name or tag that means one field in the application dictionary
    /// and another in the transport.
    FieldMismatch {
        application: (String, Tag),
        transport: (String, Tag),
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::ApplicationHeader => {
                write!(f, "the application dictionary has a header of its own")
            }
            Error::ApplicationTrailer => {
                write!(f, "the application dictionary has a trailer of its own")
            }
            Error::NoTransportHeader => write!(f, "the transport dictionary has no header"),
            Error::FieldMismatch {
                application,
                transport,
            } => write!(
                f,
                "field '{}' ({}) in the application dictionary is '{}' ({}) in the transport",
                application.0, application.1, transport.0, transport.1
            ),
        }
    }
}

impl std::error::Error for Error {}
