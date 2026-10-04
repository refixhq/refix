use crate::{MemberContext, Tag};
use std::fmt;

/// A defect found while resolving a [`crate::Spec`] into a
/// [`crate::Dictionary`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    DuplicateTag {
        tag: Tag,
    },
    /// A value listed for an int-based field that is not a FIX integer.
    NonIntegerValue {
        field: String,
        value: String,
    },
    DuplicateComponent {
        component: String,
    },
    DuplicateField {
        tag: Tag,
        first: MemberContext,
        second: MemberContext,
    },
    UnknownField {
        tag: Tag,
        context: MemberContext,
    },
    UnknownComponent {
        component: String,
        context: MemberContext,
    },
    CircularComponent {
        component: String,
    },
    EmptyGroup {
        context: MemberContext,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::DuplicateTag { tag } => {
                write!(f, "tag {tag} is defined more than once")
            }
            Error::NonIntegerValue { field, value } => {
                write!(f, "value '{value}' of field '{field}' is not an integer")
            }
            Error::DuplicateComponent { component } => {
                write!(f, "component '{component}' is defined more than once")
            }
            Error::DuplicateField { tag, first, second } if first == second => {
                write!(f, "{first} contains tag {tag} more than once")
            }
            Error::DuplicateField { tag, first, second } => {
                write!(f, "tag {tag} appears in both {first} and {second}")
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
            Error::EmptyGroup { context } => {
                write!(f, "{context} has no members")
            }
        }
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_duplicate_in_one_place_reads_as_contained_twice() {
        let context = MemberContext::Component("Top".to_owned());
        let error = Error::DuplicateField {
            tag: Tag(58),
            first: context.clone(),
            second: context,
        };

        assert_eq!(
            error.to_string(),
            "component 'Top' contains tag 58 more than once"
        );
    }

    #[test]
    fn a_duplicate_in_two_places_names_both() {
        let message = MemberContext::Message("NewOrderSingle".to_owned());
        let error = Error::DuplicateField {
            tag: Tag(448),
            first: message.clone(),
            second: message.group("NoPartyIDs"),
        };

        assert_eq!(
            error.to_string(),
            "tag 448 appears in both message 'NewOrderSingle' and group 'NoPartyIDs' in message 'NewOrderSingle'"
        );
    }
}
