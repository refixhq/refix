use crate::{MemberContext, Tag};
use std::fmt;

/// Something the resolver dropped from a [`crate::Spec`] to keep the
/// [`crate::Dictionary`] sound.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Warning {
    /// A tag listed again in a message, at any depth, after `kept`.
    DuplicateField {
        tag: Tag,
        kept: MemberContext,
        dropped: MemberContext,
    },
}

impl fmt::Display for Warning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Warning::DuplicateField { tag, kept, dropped } if kept == dropped => {
                write!(
                    f,
                    "tag {tag} is dropped from {dropped}, which lists it more than once"
                )
            }
            Warning::DuplicateField { tag, kept, dropped } => {
                write!(
                    f,
                    "tag {tag} in {dropped} is dropped, as it already appears in {kept}"
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_duplicate_in_one_place_reads_as_listed_more_than_once() {
        let context = MemberContext::Component("Top".to_owned());
        let warning = Warning::DuplicateField {
            tag: Tag(58),
            kept: context.clone(),
            dropped: context,
        };

        assert_eq!(
            warning.to_string(),
            "tag 58 is dropped from component 'Top', which lists it more than once"
        );
    }

    #[test]
    fn a_duplicate_in_two_places_names_both() {
        let message = MemberContext::Message("NewOrderSingle".to_owned());
        let warning = Warning::DuplicateField {
            tag: Tag(448),
            kept: message.clone(),
            dropped: message.group("NoPartyIDs"),
        };

        assert_eq!(
            warning.to_string(),
            "tag 448 in group 'NoPartyIDs' in message 'NewOrderSingle' is dropped, as it already \
             appears in message 'NewOrderSingle'"
        );
    }
}
