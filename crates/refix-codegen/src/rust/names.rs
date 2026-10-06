use std::collections::HashMap;

use refix_dictionary::{Dictionary, Field, MemberContext, Tag, dictionary};

use super::Error;
use super::naming::{group_name, message_module_name, method_name, variant_name};
use crate::groups::{declared_groups, instance_context, is_generated};
use crate::namespace::{Namespace, Owner};

/// Names the generated root relies on.
///
/// These are its imports, the standard names its signatures use, the crate
/// its paths start from, and `Instance`, which each group module defines
/// over the root names it imports.
const ROOT: &[&str] = &[
    "From",
    "GroupTable",
    "Instance",
    "Instances",
    "InvalidValue",
    "KnownTags",
    "MultipleValues",
    "Ok",
    "Option",
    "RawMessage",
    "Result",
    "Tag",
    "i64",
    "refix_message",
    "str",
    "u8",
];

/// The constants and methods every message defines.
const MESSAGE: &[&str] = &["KNOWN_TAGS", "MSG_TYPE", "from_raw", "raw"];

/// The methods every group instance defines.
const INSTANCE: &[&str] = &["raw"];

/// Every name the generated Rust uses, settled before anything is emitted.
pub(super) struct Names {
    enums: HashMap<Tag, EnumNames>,
    accessors: HashMap<Tag, String>,
    groups: HashMap<MemberContext, String>,
    message_modules: HashMap<String, String>,
}

struct EnumNames {
    type_name: String,
    variants: Vec<String>,
}

impl Names {
    /// Names everything generated from `dictionary`, failing when two
    /// things would share a name.
    pub(super) fn new(dictionary: &Dictionary) -> Result<Self, Error> {
        let mut names = Names {
            enums: HashMap::new(),
            accessors: HashMap::new(),
            groups: HashMap::new(),
            message_modules: HashMap::new(),
        };
        let mut root = Namespace::with_generated(ROOT);
        for message in dictionary.messages() {
            root.claim(message.name(), Owner::Message(message.name().to_owned()))?;
        }
        for message in dictionary.messages() {
            names.name_message_module(message, &mut root)?;
        }
        for group in declared_groups(dictionary) {
            if matches!(group.declared_in(), MemberContext::Component(_)) {
                let name = group_name(group)?;
                root.claim(&name, Owner::Group(instance_context(group)))?;
                names.groups.insert(instance_context(group), name);
            }
        }
        for field in dictionary.fields() {
            if !field.values.is_empty() {
                root.claim(&field.name, Owner::Enum(field.name.clone()))?;
                names.name_enum(field)?;
            }
        }
        for message in dictionary.messages() {
            names.name_accessors(message.members(), Namespace::with_generated(MESSAGE))?;
        }
        for group in declared_groups(dictionary) {
            if is_generated(group) {
                names.name_accessors(group.members(), Namespace::with_generated(INSTANCE))?;
            }
        }
        Ok(names)
    }

    /// Names the module holding the groups a message declares itself, if it
    /// declares any.
    fn name_message_module(
        &mut self,
        message: dictionary::Message<'_>,
        root: &mut Namespace,
    ) -> Result<(), Error> {
        let groups: Vec<dictionary::Group<'_>> = message
            .members()
            .filter_map(|member| match member {
                dictionary::Member::Group(group)
                    if matches!(group.declared_in(), MemberContext::Message(_)) =>
                {
                    Some(group)
                }
                _ => None,
            })
            .collect();
        if groups.is_empty() {
            return Ok(());
        }
        let module = message_module_name(message.name())?;
        root.claim(&module, Owner::Message(message.name().to_owned()))?;
        let mut modules = Namespace::default();
        for group in groups {
            let name = group_name(group)?;
            modules.claim(&name, Owner::Group(instance_context(group)))?;
            self.groups.insert(instance_context(group), name);
        }
        self.message_modules
            .insert(message.name().to_owned(), module);
        Ok(())
    }

    fn name_enum(&mut self, field: &Field) -> Result<(), Error> {
        let mut variants = Namespace::with_generated(&["Unrecognized"]);
        let names = field
            .values
            .iter()
            .map(|value| {
                let name = variant_name(field, value)?;
                variants.claim(
                    &name,
                    Owner::Value {
                        field: field.name.clone(),
                        code: value.value.clone(),
                        description: value.description.clone(),
                    },
                )?;
                Ok(name)
            })
            .collect::<Result<Vec<_>, Error>>()?;
        self.enums.insert(
            field.tag,
            EnumNames {
                type_name: field.name.clone(),
                variants: names,
            },
        );
        Ok(())
    }

    /// Names the accessors of a message or group instance.
    fn name_accessors<'a>(
        &mut self,
        members: impl Iterator<Item = dictionary::Member<'a>>,
        mut namespace: Namespace,
    ) -> Result<(), Error> {
        for member in members {
            match member {
                dictionary::Member::Field { field, .. } => {
                    let name = method_name(field)?;
                    namespace.claim(&name, Owner::Field(field.name.clone()))?;
                    self.accessors.insert(field.tag, name);
                }
                dictionary::Member::Group(group) if is_generated(group) => {
                    namespace.claim(self.group(group), Owner::Group(instance_context(group)))?;
                }
                dictionary::Member::Group(_) => {}
            }
        }
        Ok(())
    }

    /// The type of a field's enum.
    pub(super) fn enum_type(&self, field: &Field) -> &str {
        &self.enums[&field.tag].type_name
    }

    /// The variants of a field's enum, one per value.
    pub(super) fn variants(&self, field: &Field) -> &[String] {
        &self.enums[&field.tag].variants
    }

    /// The method reading a field.
    pub(super) fn accessor(&self, field: &Field) -> &str {
        &self.accessors[&field.tag]
    }

    /// A group's module, which is also the name of its accessor.
    pub(super) fn group(&self, group: dictionary::Group<'_>) -> &str {
        &self.groups[&instance_context(group)]
    }

    /// The module holding the groups a message declares itself.
    pub(super) fn message_module(&self, message: &str) -> &str {
        &self.message_modules[message]
    }

    /// The path of a group's module from the generated root.
    pub(super) fn group_path(&self, group: dictionary::Group<'_>) -> String {
        let name = self.group(group);
        match group.declared_in() {
            MemberContext::Message(message) => format!("{}::{name}", self.message_module(message)),
            _ => name.to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Names;
    use crate::rust::Error;
    use crate::test_utils::message_with;
    use crate::{NameClash, Owner};
    use refix_dictionary::{
        Category, Component, ComponentRef, DataType, EnumValue, Field, FieldRef, Group, Member,
        MemberContext, Message, Protocol, Spec, Tag, Version,
    };

    fn field(name: &str, tag: u32, data_type: DataType) -> Field {
        Field {
            name: name.to_owned(),
            tag: Tag(tag),
            data_type,
            values: vec![],
        }
    }

    fn with_values(mut field: Field, values: &[(&str, &str)]) -> Field {
        field.values = values
            .iter()
            .map(|(value, description)| EnumValue {
                value: (*value).to_owned(),
                description: (*description).to_owned(),
            })
            .collect();
        field
    }

    fn clash(fields: Vec<Field>) -> NameClash {
        match Names::new(&message_with(fields)) {
            Err(Error::NameClash(clash)) => *clash,
            Err(other) => panic!("expected a name clash, found {other:?}"),
            Ok(_) => panic!("expected a name clash"),
        }
    }

    #[test]
    fn an_enum_named_like_a_message_clashes() {
        let field = with_values(
            field("NewOrderSingle", 9000, DataType::Char),
            &[("1", "FIRST")],
        );
        assert_eq!(
            clash(vec![field]),
            NameClash {
                name: "NewOrderSingle".to_owned(),
                first: Owner::Message("NewOrderSingle".to_owned()),
                second: Owner::Enum("NewOrderSingle".to_owned()),
            }
        );
    }

    #[test]
    fn an_enum_named_like_a_name_the_generated_code_uses_clashes() {
        let field = with_values(field("Result", 9000, DataType::Char), &[("1", "FIRST")]);
        assert_eq!(
            clash(vec![field]),
            NameClash {
                name: "Result".to_owned(),
                first: Owner::Generated,
                second: Owner::Enum("Result".to_owned()),
            }
        );
    }

    #[test]
    fn two_values_named_alike_clash() {
        let field = with_values(
            field("BenchmarkCurveName", 221, DataType::String),
            &[("Euribor", "EURIBOR"), ("EURIBOR", "EURIBOR")],
        );
        let value = |code: &str| Owner::Value {
            field: "BenchmarkCurveName".to_owned(),
            code: code.to_owned(),
            description: "EURIBOR".to_owned(),
        };
        assert_eq!(
            clash(vec![field]),
            NameClash {
                name: "Euribor".to_owned(),
                first: value("Euribor"),
                second: value("EURIBOR"),
            }
        );
    }

    #[test]
    fn a_value_named_like_the_catch_all_clashes() {
        let field = with_values(
            field("OrdType", 40, DataType::Char),
            &[("1", "UNRECOGNIZED")],
        );
        assert_eq!(clash(vec![field]).first, Owner::Generated);
    }

    #[test]
    fn two_fields_with_one_accessor_name_clash() {
        assert_eq!(
            clash(vec![
                field("OrderQty", 38, DataType::Int),
                field("OrderQTY", 9000, DataType::Int),
            ]),
            NameClash {
                name: "order_qty".to_owned(),
                first: Owner::Field("OrderQty".to_owned()),
                second: Owner::Field("OrderQTY".to_owned()),
            }
        );
    }

    #[test]
    fn an_accessor_named_like_a_generated_method_clashes() {
        assert_eq!(
            clash(vec![field("Raw", 9000, DataType::String)]),
            NameClash {
                name: "raw".to_owned(),
                first: Owner::Generated,
                second: Owner::Field("Raw".to_owned()),
            }
        );
    }

    #[test]
    fn a_message_module_and_a_group_module_clash() {
        let field_ref = |tag| {
            Member::Field(FieldRef {
                tag: Tag(tag),
                is_required: false,
            })
        };
        let group = |count_tag, members| {
            Member::Group(Group {
                count_tag: Tag(count_tag),
                is_required: false,
                members,
            })
        };
        let message = |name: &str, msg_type: &str, members| Message {
            name: name.to_owned(),
            msg_type: msg_type.to_owned(),
            members,
            category: Category::App,
        };
        let dictionary = Spec {
            version: Version {
                protocol: Protocol::Fix,
                major: 4,
                minor: 4,
                service_pack: 0,
            },
            messages: vec![
                message("Parties", "X", vec![group(384, vec![field_ref(372)])]),
                message(
                    "NewOrderSingle",
                    "D",
                    vec![Member::Component(ComponentRef {
                        name: "Parties".to_owned(),
                        is_required: false,
                    })],
                ),
            ],
            fields: vec![
                field("NoMsgTypes", 384, DataType::NumInGroup),
                field("RefMsgType", 372, DataType::String),
                field("NoPartyIDs", 453, DataType::NumInGroup),
                field("PartyID", 448, DataType::String),
            ],
            components: vec![Component {
                name: "Parties".to_owned(),
                members: vec![group(453, vec![field_ref(448)])],
            }],
        }
        .resolve()
        .unwrap();

        let Err(Error::NameClash(clash)) = Names::new(&dictionary) else {
            panic!("expected a name clash");
        };
        assert_eq!(
            *clash,
            NameClash {
                name: "parties".to_owned(),
                first: Owner::Message("Parties".to_owned()),
                second: Owner::Group(
                    MemberContext::Component("Parties".to_owned()).group("NoPartyIDs")
                ),
            }
        );
    }

    #[test]
    fn a_clash_names_the_language() {
        let field = with_values(field("Result", 9000, DataType::Char), &[("1", "FIRST")]);
        let error = Names::new(&message_with(vec![field])).err().unwrap();
        assert_eq!(
            error.to_string(),
            "rust name `Result` is taken by both the generated code and the enum of field 'Result'"
        );
    }
}
