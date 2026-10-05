use crate::Tag;
use std::fmt;

/// A group table built at runtime that breaks one of the table rules.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GroupTableError {
    UnsortedMembers,
    DelimiterNotMember { delimiter: Tag },
    NestedCountNotMember { count_tag: Tag },
}

impl fmt::Display for GroupTableError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GroupTableError::UnsortedMembers => {
                write!(f, "group members must be sorted and unique")
            }
            GroupTableError::DelimiterNotMember { delimiter } => {
                write!(f, "group members must include the delimiter {delimiter}")
            }
            GroupTableError::NestedCountNotMember { count_tag } => {
                write!(
                    f,
                    "group members must include the nested count tag {count_tag}"
                )
            }
        }
    }
}

impl std::error::Error for GroupTableError {}
