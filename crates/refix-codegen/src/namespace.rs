mod error;

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fmt;

use refix_dictionary::MemberContext;

pub use error::NameClash;

/// What a generated name stands for.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Owner {
    /// A name the generated code defines or imports for itself.
    Generated,
    /// A field's accessor.
    Field(String),
    /// The enum of a field's values.
    Enum(String),
    /// A message's type or module.
    Message(String),
    /// A group's type, module or accessor, by the context of its instances.
    Group(MemberContext),
    /// One of a field's values.
    Value {
        field: String,
        code: String,
        description: String,
    },
}

impl fmt::Display for Owner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Owner::Generated => write!(f, "the generated code"),
            Owner::Field(field) => write!(f, "field '{field}'"),
            Owner::Enum(field) => write!(f, "the enum of field '{field}'"),
            Owner::Message(message) => write!(f, "message '{message}'"),
            Owner::Group(context) => write!(f, "{context}"),
            Owner::Value {
                field,
                code,
                description,
            } => write!(f, "value '{description}' (code {code}) of field '{field}'"),
        }
    }
}

/// The names taken in one scope of the generated code.
#[derive(Debug, Default)]
pub(crate) struct Namespace {
    owners: HashMap<String, Owner>,
}

impl Namespace {
    /// A namespace in which the generated code already takes `names`.
    pub(crate) fn with_generated(names: &[&str]) -> Self {
        let owners = names
            .iter()
            .map(|name| ((*name).to_owned(), Owner::Generated))
            .collect();
        Self { owners }
    }

    /// Takes `name` for `owner`, unless something else already has it.
    pub(crate) fn claim(&mut self, name: &str, owner: Owner) -> Result<(), Box<NameClash>> {
        match self.owners.entry(name.to_owned()) {
            Entry::Occupied(taken) => Err(Box::new(NameClash {
                name: name.to_owned(),
                first: taken.get().clone(),
                second: owner,
            })),
            Entry::Vacant(free) => {
                free.insert(owner);
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{NameClash, Namespace, Owner};
    use refix_dictionary::MemberContext;

    #[test]
    fn a_free_name_can_be_claimed() {
        let mut namespace = Namespace::default();
        assert!(
            namespace
                .claim("OrdType", Owner::Enum("OrdType".to_owned()))
                .is_ok()
        );
    }

    #[test]
    fn a_taken_name_reports_both_owners() {
        let mut namespace = Namespace::default();
        namespace
            .claim(
                "SecurityStatus",
                Owner::Message("SecurityStatus".to_owned()),
            )
            .unwrap();

        assert_eq!(
            namespace
                .claim("SecurityStatus", Owner::Enum("SecurityStatus".to_owned()))
                .unwrap_err(),
            Box::new(NameClash {
                name: "SecurityStatus".to_owned(),
                first: Owner::Message("SecurityStatus".to_owned()),
                second: Owner::Enum("SecurityStatus".to_owned()),
            })
        );
    }

    #[test]
    fn generated_names_are_taken_from_the_start() {
        let mut namespace = Namespace::with_generated(&["raw"]);
        assert_eq!(
            namespace
                .claim("raw", Owner::Field("Raw".to_owned()))
                .unwrap_err()
                .first,
            Owner::Generated
        );
    }

    #[test]
    fn a_clash_names_the_name_and_both_owners() {
        let clash = NameClash {
            name: "Euribor".to_owned(),
            first: Owner::Value {
                field: "BenchmarkCurveName".to_owned(),
                code: "Euribor".to_owned(),
                description: "EURIBOR".to_owned(),
            },
            second: Owner::Value {
                field: "BenchmarkCurveName".to_owned(),
                code: "EURIBOR".to_owned(),
                description: "EURIBOR".to_owned(),
            },
        };
        assert_eq!(
            clash.to_string(),
            "`Euribor` is taken by both value 'EURIBOR' (code Euribor) of field \
             'BenchmarkCurveName' and value 'EURIBOR' (code EURIBOR) of field 'BenchmarkCurveName'"
        );
    }

    #[test]
    fn owners_describe_where_a_name_comes_from() {
        let context = MemberContext::Component("Parties".to_owned()).group("NoPartyIDs");
        assert_eq!(Owner::Generated.to_string(), "the generated code");
        assert_eq!(Owner::Field("Raw".to_owned()).to_string(), "field 'Raw'");
        assert_eq!(
            Owner::Enum("RateSource".to_owned()).to_string(),
            "the enum of field 'RateSource'"
        );
        assert_eq!(
            Owner::Message("Logon".to_owned()).to_string(),
            "message 'Logon'"
        );
        assert_eq!(
            Owner::Group(context).to_string(),
            "group 'NoPartyIDs' in component 'Parties'"
        );
    }
}
