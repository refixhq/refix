mod error;

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fmt;

use refix_dictionary::MemberContext;

use crate::Warning;
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

    /// Takes the name of a field's enum.
    ///
    /// The enum yields to a message or group of its name, taking `Enum`
    /// after it, with a warning.
    pub(crate) fn claim_enum(
        &mut self,
        field: &str,
        warnings: &mut Vec<Warning>,
    ) -> Result<String, Box<NameClash>> {
        let owner = Owner::Enum(field.to_owned());
        let clash = match self.claim(field, owner.clone()) {
            Ok(()) => return Ok(field.to_owned()),
            Err(clash) if matches!(clash.first, Owner::Message(_) | Owner::Group(_)) => clash,
            Err(clash) => return Err(clash),
        };
        let name = format!("{field}Enum");
        self.claim(&name, owner)?;
        warnings.push(Warning::Renamed {
            clash,
            name: name.clone(),
        });
        Ok(name)
    }

    /// Takes the name of one of a field's values.
    ///
    /// A value named like an earlier one of the field is numbered from 2 by
    /// `numbered`, with a warning.
    pub(crate) fn claim_value(
        &mut self,
        name: &str,
        owner: Owner,
        numbered: impl Fn(&str, u32) -> String,
        warnings: &mut Vec<Warning>,
    ) -> Result<String, Box<NameClash>> {
        let clash = match self.claim(name, owner.clone()) {
            Ok(()) => return Ok(name.to_owned()),
            Err(clash) if matches!(clash.first, Owner::Value { .. }) => clash,
            Err(clash) => return Err(clash),
        };
        let mut number = 2;
        let renamed = loop {
            let candidate = numbered(name, number);
            if self.claim(&candidate, owner.clone()).is_ok() {
                break candidate;
            }
            number += 1;
        };
        warnings.push(Warning::Renamed {
            clash,
            name: renamed.clone(),
        });
        Ok(renamed)
    }
}

#[cfg(test)]
mod tests {
    use super::{NameClash, Namespace, Owner};
    use crate::Warning;
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
    fn an_enum_yields_to_a_message() {
        let mut namespace = Namespace::default();
        namespace
            .claim(
                "SecurityStatus",
                Owner::Message("SecurityStatus".to_owned()),
            )
            .unwrap();
        let mut warnings = Vec::new();

        let name = namespace
            .claim_enum("SecurityStatus", &mut warnings)
            .unwrap();

        assert_eq!(name, "SecurityStatusEnum");
        assert_eq!(
            warnings,
            vec![Warning::Renamed {
                clash: Box::new(NameClash {
                    name: "SecurityStatus".to_owned(),
                    first: Owner::Message("SecurityStatus".to_owned()),
                    second: Owner::Enum("SecurityStatus".to_owned()),
                }),
                name: "SecurityStatusEnum".to_owned(),
            }]
        );
    }

    #[test]
    fn an_enum_yields_to_a_group() {
        let context = MemberContext::Component("RateSource".to_owned()).group("NoRateSources");
        let mut namespace = Namespace::default();
        namespace
            .claim("RateSource", Owner::Group(context))
            .unwrap();

        let name = namespace.claim_enum("RateSource", &mut Vec::new()).unwrap();

        assert_eq!(name, "RateSourceEnum");
    }

    #[test]
    fn an_enum_does_not_yield_to_the_generated_code() {
        let mut namespace = Namespace::with_generated(&["Result"]);
        let mut warnings = Vec::new();

        let clash = namespace.claim_enum("Result", &mut warnings).unwrap_err();

        assert_eq!(clash.first, Owner::Generated);
        assert!(warnings.is_empty());
    }

    #[test]
    fn a_yielding_enum_clashes_when_its_new_name_is_taken() {
        let mut namespace = Namespace::default();
        for message in ["SecurityStatus", "SecurityStatusEnum"] {
            namespace
                .claim(message, Owner::Message(message.to_owned()))
                .unwrap();
        }

        let clash = namespace
            .claim_enum("SecurityStatus", &mut Vec::new())
            .unwrap_err();

        assert_eq!(
            clash,
            Box::new(NameClash {
                name: "SecurityStatusEnum".to_owned(),
                first: Owner::Message("SecurityStatusEnum".to_owned()),
                second: Owner::Enum("SecurityStatus".to_owned()),
            })
        );
    }

    fn value(code: &str) -> Owner {
        Owner::Value {
            field: "BenchmarkCurveName".to_owned(),
            code: code.to_owned(),
            description: "EURIBOR".to_owned(),
        }
    }

    fn numbered(name: &str, number: u32) -> String {
        format!("{name}{number}")
    }

    #[test]
    fn values_named_alike_are_numbered_in_order() {
        let mut namespace = Namespace::default();
        let mut warnings = Vec::new();

        let names: Vec<String> = ["Euribor", "EURIBOR", "euribor"]
            .into_iter()
            .map(|code| {
                namespace
                    .claim_value("Euribor", value(code), numbered, &mut warnings)
                    .unwrap()
            })
            .collect();

        assert_eq!(names, ["Euribor", "Euribor2", "Euribor3"]);
        assert_eq!(warnings.len(), 2);
        assert_eq!(
            warnings[1],
            Warning::Renamed {
                clash: Box::new(NameClash {
                    name: "Euribor".to_owned(),
                    first: value("Euribor"),
                    second: value("euribor"),
                }),
                name: "Euribor3".to_owned(),
            }
        );
    }

    #[test]
    fn a_numbered_value_skips_a_taken_number() {
        let mut namespace = Namespace::default();
        namespace.claim("Euribor", value("Euribor")).unwrap();
        namespace.claim("Euribor2", value("Euribor2")).unwrap();

        let name = namespace
            .claim_value("Euribor", value("EURIBOR"), numbered, &mut Vec::new())
            .unwrap();

        assert_eq!(name, "Euribor3");
    }

    #[test]
    fn a_value_is_not_numbered_past_the_generated_code() {
        let mut namespace = Namespace::with_generated(&["Unrecognized"]);

        let clash = namespace
            .claim_value("Unrecognized", value("U"), numbered, &mut Vec::new())
            .unwrap_err();

        assert_eq!(clash.first, Owner::Generated);
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
