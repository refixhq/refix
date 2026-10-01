use crate::{MemberContext, Tag};
use std::fmt;

/// A defect found while resolving a [`crate::Spec`] into a
/// [`crate::Dictionary`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    DuplicateTag {
        tag: Tag,
    },
    DuplicateComponent {
        component: String,
    },
    UnknownField {
        context: MemberContext,
        tag: Tag,
    },
    UnknownComponent {
        context: MemberContext,
        component: String,
    },
    CircularComponent {
        component: String,
    },
    DuplicateField {
        context: MemberContext,
        tag: Tag,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::DuplicateTag { tag } => {
                write!(f, "tag {tag} is defined more than once")
            }
            Error::DuplicateComponent { component } => {
                write!(f, "component '{component}' is defined more than once")
            }
            Error::UnknownField { context, tag } => {
                write!(f, "{context} references unknown tag {tag}")
            }
            Error::UnknownComponent { context, component } => {
                write!(f, "{context} references unknown component '{component}'")
            }
            Error::CircularComponent { component } => {
                write!(f, "component '{component}' is part of a reference cycle")
            }
            Error::DuplicateField { context, tag } => {
                write!(f, "{context} contains tag {tag} more than once")
            }
        }
    }
}

impl std::error::Error for Error {}
