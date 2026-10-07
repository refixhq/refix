use refix_dictionary::MemberContext;

use crate::NameClash;

/// Where the generated code departs from the dictionary.
#[derive(Debug, Eq, PartialEq)]
pub enum Warning {
    /// A construct the generator recognised but does not generate code for yet.
    UnsupportedGroup { context: MemberContext },
    /// The second owner of a clashing name, given `name` in its place.
    Renamed { clash: Box<NameClash>, name: String },
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Warning;
    use crate::{NameClash, Owner};

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
}
