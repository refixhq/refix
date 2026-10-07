use refix_dictionary::MemberContext;

use crate::{Language, NameClash};

/// Where the generated code departs from the dictionary.
#[derive(Debug, Eq, PartialEq)]
pub enum Warning {
    /// A construct the generator recognised but does not generate code for yet.
    UnsupportedGroup { context: MemberContext },
    /// The second owner of a clashing name, given `name` in its place.
    Renamed {
        language: Language,
        clash: Box<NameClash>,
        name: String,
    },
}

impl std::fmt::Display for Warning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Warning::UnsupportedGroup { context } => {
                write!(f, "{context} is not generated yet")
            }
            Warning::Renamed {
                language,
                clash,
                name,
            } => write!(
                f,
                "{language} name `{}` is taken by {}, so {} is named `{name}`",
                clash.name, clash.first, clash.second
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Warning;
    use crate::{Language, NameClash, Owner};

    #[test]
    fn a_rename_names_the_language_and_both_owners() {
        let warning = Warning::Renamed {
            language: Language::Rust,
            clash: Box::new(NameClash {
                name: "SecurityStatus".to_owned(),
                first: Owner::Message("SecurityStatus".to_owned()),
                second: Owner::Enum("SecurityStatus".to_owned()),
            }),
            name: "SecurityStatusEnum".to_owned(),
        };
        assert_eq!(
            warning.to_string(),
            "rust name `SecurityStatus` is taken by message 'SecurityStatus', so the enum of \
             field 'SecurityStatus' is named `SecurityStatusEnum`"
        );
    }
}
