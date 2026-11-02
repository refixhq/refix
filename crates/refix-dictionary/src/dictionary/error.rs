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
    /// Two values listed under one code of a field, `1` and `01` being one
    /// code on an int-based field.
    RepeatedCode {
        field: String,
        first: String,
        second: String,
    },
    DuplicateComponent {
        component: String,
    },
    /// Two messages declaring the same MsgType.
    DuplicateMsgType {
        msg_type: String,
        first: String,
        second: String,
    },
    /// A length tag on a field that is not a data field.
    UnexpectedLengthTag {
        field: String,
        tag: Tag,
    },
    UnknownLengthTag {
        field: String,
        tag: Tag,
    },
    /// A data field taking its length from a field not typed LENGTH.
    NonLengthField {
        field: String,
        length_field: String,
    },
    /// Two data fields taking their length from one field.
    SharedLengthField {
        length_field: String,
        first: String,
        second: String,
    },
    /// A data field used without a length tag.
    MissingLengthTag {
        field: String,
        context: MemberContext,
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
            Error::RepeatedCode {
                field,
                first,
                second,
            } if first == second => {
                write!(f, "field '{field}' lists code {first} more than once")
            }
            Error::RepeatedCode {
                field,
                first,
                second,
            } => {
                write!(
                    f,
                    "field '{field}' lists both {first} and {second}, which are the same number"
                )
            }
            Error::DuplicateComponent { component } => {
                write!(f, "component '{component}' is defined more than once")
            }
            Error::DuplicateMsgType {
                msg_type,
                first,
                second,
            } => {
                write!(
                    f,
                    "messages '{first}' and '{second}' share MsgType '{msg_type}'"
                )
            }
            Error::UnexpectedLengthTag { field, tag } => {
                write!(
                    f,
                    "field '{field}' takes its length from tag {tag} but is not a data field"
                )
            }
            Error::UnknownLengthTag { field, tag } => {
                write!(f, "field '{field}' takes its length from unknown tag {tag}")
            }
            Error::NonLengthField {
                field,
                length_field,
            } => {
                write!(
                    f,
                    "field '{field}' takes its length from '{length_field}', which is not a LENGTH field"
                )
            }
            Error::SharedLengthField {
                length_field,
                first,
                second,
            } => {
                write!(
                    f,
                    "fields '{first}' and '{second}' both take their length from '{length_field}'"
                )
            }
            Error::MissingLengthTag { field, context } => {
                write!(f, "data field '{field}' in {context} has no length field")
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
    fn a_shared_msg_type_names_both_messages() {
        let error = Error::DuplicateMsgType {
            msg_type: "D".to_owned(),
            first: "NewOrderSingle".to_owned(),
            second: "NewOrderCross".to_owned(),
        };

        assert_eq!(
            error.to_string(),
            "messages 'NewOrderSingle' and 'NewOrderCross' share MsgType 'D'"
        );
    }

    #[test]
    fn a_shared_length_field_names_both_data_fields() {
        let error = Error::SharedLengthField {
            length_field: "RawDataLength".to_owned(),
            first: "RawData".to_owned(),
            second: "SecureData".to_owned(),
        };

        assert_eq!(
            error.to_string(),
            "fields 'RawData' and 'SecureData' both take their length from 'RawDataLength'"
        );
    }

    #[test]
    fn a_repeated_code_reads_as_listed_twice() {
        let error = Error::RepeatedCode {
            field: "MatchType".to_owned(),
            first: "M1".to_owned(),
            second: "M1".to_owned(),
        };

        assert_eq!(
            error.to_string(),
            "field 'MatchType' lists code M1 more than once"
        );
    }

    #[test]
    fn a_code_spelled_twice_names_both_spellings() {
        let error = Error::RepeatedCode {
            field: "PriceType".to_owned(),
            first: "1".to_owned(),
            second: "01".to_owned(),
        };

        assert_eq!(
            error.to_string(),
            "field 'PriceType' lists both 1 and 01, which are the same number"
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
