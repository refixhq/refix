use refix_dictionary::{DataType, MemberContext};

use crate::NameClash;

/// Where the generated code departs from the dictionary.
#[derive(Debug, Eq, PartialEq)]
pub enum Warning {
    /// A construct the generator recognised but does not generate code for yet.
    UnsupportedGroup { context: MemberContext },
    /// The second owner of a clashing name, given `name` in its place.
    Renamed { clash: Box<NameClash>, name: String },
    /// A field read as raw bytes, as the generator does not interpret its type.
    UnclaimedType { field: String, data_type: DataType },
}

impl std::fmt::Display for Warning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Warning::UnsupportedGroup { context } => {
                write!(f, "{context} is not generated yet")
            }
            Warning::Renamed { clash, name } => write!(
                f,
                "`{}` is taken by {}, so {} is named `{name}`",
                clash.name, clash.first, clash.second
            ),
            Warning::UnclaimedType { field, data_type } => write!(
                f,
                "field '{field}' has type {data_type}, which is not interpreted, so it reads as \
                 raw bytes"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Warning;
    use crate::{NameClash, Owner};
    use refix_dictionary::DataType;

    #[test]
    fn a_rename_names_both_owners_and_the_new_name() {
        let warning = Warning::Renamed {
            clash: Box::new(NameClash {
                name: "SecurityStatus".to_owned(),
                first: Owner::Message("SecurityStatus".to_owned()),
                second: Owner::Enum("SecurityStatus".to_owned()),
            }),
            name: "SecurityStatusEnum".to_owned(),
        };
        assert_eq!(
            warning.to_string(),
            "`SecurityStatus` is taken by message 'SecurityStatus', so the enum of field \
             'SecurityStatus' is named `SecurityStatusEnum`"
        );
    }

    #[test]
    fn an_unclaimed_type_names_the_field_and_its_type() {
        let warning = Warning::UnclaimedType {
            field: "RatioQty".to_owned(),
            data_type: DataType::Other("QUANTITY".to_owned()),
        };
        assert_eq!(
            warning.to_string(),
            "field 'RatioQty' has type QUANTITY, which is not interpreted, so it reads as raw bytes"
        );
    }
}
