use std::collections::HashMap;

use refix_dictionary::{Dictionary, Field, MemberContext, Tag, dictionary};

use super::Error;
use super::naming::{group_property_name, member_name, numbered_member, property_name};
use crate::groups::{declared_groups, instance_context, is_generated};
use crate::namespace::{Namespace, Owner};
use crate::naming::group_source_name;
use crate::{Language, Warning};

/// The imports and builtins the generated module relies on.
const MODULE: &[&str] = &[
    "ClassVar",
    "GroupTable",
    "KnownTags",
    "RawMessage",
    "Unrecognized",
    "annotations",
    "bytes",
    "cached_property",
    "enum",
    "from_value",
    "int",
    "property",
    "refix",
    "str",
    "tuple",
];

/// The members every message class defines, and the decorators its body
/// looks up.
///
/// A property named like a decorator would replace it for the properties
/// after it. Annotations are not looked up in the class body, as the module
/// defers them.
const MESSAGE: &[&str] = &[
    "KNOWN_TAGS",
    "MSG_TYPE",
    "__init__",
    "_raw",
    "cached_property",
    "property",
    "raw",
];

/// The members every group instance class defines, and the decorators its
/// body looks up.
const INSTANCE: &[&str] = &["__init__", "_scope", "cached_property", "property", "raw"];

/// Every name the generated Python uses, settled before anything is emitted.
pub(super) struct Names {
    enums: HashMap<Tag, EnumNames>,
    properties: HashMap<Tag, String>,
    groups: HashMap<MemberContext, GroupNames>,
}

struct EnumNames {
    class: String,
    members: Vec<String>,
}

struct GroupNames {
    class: String,
    property: String,
}

impl Names {
    /// Names everything generated from `dictionary`.
    ///
    /// A clash a rule settles is renamed with a warning; any other fails.
    pub(super) fn new(dictionary: &Dictionary, warnings: &mut Vec<Warning>) -> Result<Self, Error> {
        let mut names = Names {
            enums: HashMap::new(),
            properties: HashMap::new(),
            groups: HashMap::new(),
        };
        let mut module = Namespace::with_generated(MODULE);
        for message in dictionary.messages() {
            module.claim(message.name(), Owner::Message(message.name().to_owned()))?;
        }
        // A message's own group class nests in the message's class, so only
        // a shared group's class is named in the module.
        for group in declared_groups(dictionary) {
            if is_generated(group) {
                let class = names.name_group(group);
                if matches!(group.declared_in(), MemberContext::Component(_)) {
                    module.claim(&class, Owner::Group(instance_context(group)))?;
                }
            }
        }
        for field in dictionary.fields() {
            if !field.values.is_empty() {
                let class = module.claim_enum(&field.name, Language::Python, warnings)?;
                names.name_enum(field, class, warnings)?;
            }
        }
        for message in dictionary.messages() {
            let mut members = Namespace::with_generated(MESSAGE);
            for member in message.members() {
                if let dictionary::Member::Group(group) = member
                    && matches!(group.declared_in(), MemberContext::Message(_))
                {
                    members.claim(
                        names.group_class(group),
                        Owner::Group(instance_context(group)),
                    )?;
                }
            }
            names.name_properties(message.members(), members)?;
        }
        for group in declared_groups(dictionary) {
            if is_generated(group) {
                names.name_properties(group.members(), Namespace::with_generated(INSTANCE))?;
            }
        }
        Ok(names)
    }

    /// Names a group's class and property, returning the class.
    fn name_group(&mut self, group: dictionary::Group<'_>) -> String {
        let class = group_source_name(group).to_owned();
        let property = group_property_name(group);
        self.groups.insert(
            instance_context(group),
            GroupNames {
                class: class.clone(),
                property,
            },
        );
        class
    }

    fn name_enum(
        &mut self,
        field: &Field,
        class: String,
        warnings: &mut Vec<Warning>,
    ) -> Result<(), Error> {
        let mut members = Namespace::default();
        let names = field
            .values
            .iter()
            .map(|value| {
                let name = member_name(field, value)?;
                let owner = Owner::Value {
                    field: field.name.clone(),
                    code: value.value.clone(),
                    description: value.description.clone(),
                };
                Ok(members.claim_value(
                    &name,
                    owner,
                    numbered_member,
                    Language::Python,
                    warnings,
                )?)
            })
            .collect::<Result<Vec<_>, Error>>()?;
        self.enums.insert(
            field.tag,
            EnumNames {
                class,
                members: names,
            },
        );
        Ok(())
    }

    /// Names the properties of a message or group instance.
    fn name_properties<'a>(
        &mut self,
        members: impl Iterator<Item = dictionary::Member<'a>>,
        mut namespace: Namespace,
    ) -> Result<(), Error> {
        for member in members {
            match member {
                dictionary::Member::Field { field, .. } => {
                    let name = property_name(field);
                    namespace.claim(&name, Owner::Field(field.name.clone()))?;
                    self.properties.insert(field.tag, name);
                }
                dictionary::Member::Group(group) if is_generated(group) => {
                    namespace.claim(
                        self.group_property(group),
                        Owner::Group(instance_context(group)),
                    )?;
                }
                dictionary::Member::Group(_) => {}
            }
        }
        Ok(())
    }

    /// The class of a field's enum.
    pub(super) fn enum_class(&self, field: &Field) -> &str {
        &self.enums[&field.tag].class
    }

    /// The members of a field's enum, one per value.
    pub(super) fn enum_members(&self, field: &Field) -> &[String] {
        &self.enums[&field.tag].members
    }

    /// The property reading a field.
    pub(super) fn property(&self, field: &Field) -> &str {
        &self.properties[&field.tag]
    }

    /// A group's class.
    pub(super) fn group_class(&self, group: dictionary::Group<'_>) -> &str {
        &self.groups[&instance_context(group)].class
    }

    /// The property reading a group's instances.
    pub(super) fn group_property(&self, group: dictionary::Group<'_>) -> &str {
        &self.groups[&instance_context(group)].property
    }

    /// The path of a group's class from the module.
    ///
    /// A message's own group sits under the message's class.
    pub(super) fn group_path(&self, group: dictionary::Group<'_>) -> String {
        let class = self.group_class(group);
        match group.declared_in() {
            MemberContext::Message(message) => format!("{message}.{class}"),
            _ => class.to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Names;
    use crate::python::Error;
    use crate::test_utils::message_with;
    use crate::{Language, NameClash, Owner, Warning};
    use refix_dictionary::{
        Category, Component, ComponentRef, DataType, Dictionary, EnumValue, Field, FieldRef, Group,
        Member, MemberContext, Message, Protocol, Spec, Tag, Version,
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

    fn field_ref(tag: u32) -> Member {
        Member::Field(FieldRef {
            tag: Tag(tag),
            is_required: false,
        })
    }

    fn group(count_tag: u32, members: Vec<Member>) -> Member {
        Member::Group(Group {
            count_tag: Tag(count_tag),
            is_required: false,
            members,
        })
    }

    fn message(name: &str, msg_type: &str, members: Vec<Member>) -> Message {
        Message {
            name: name.to_owned(),
            msg_type: msg_type.to_owned(),
            members,
            category: Category::App,
        }
    }

    fn dictionary(
        messages: Vec<Message>,
        fields: Vec<Field>,
        components: Vec<Component>,
    ) -> Dictionary {
        Spec {
            version: Version {
                protocol: Protocol::Fix,
                major: 5,
                minor: 0,
                service_pack: 2,
            },
            messages,
            fields,
            components,
        }
        .resolve()
        .unwrap()
    }

    fn clash_in(dictionary: &Dictionary) -> NameClash {
        match Names::new(dictionary, &mut Vec::new()) {
            Err(Error::NameClash(clash)) => *clash,
            Err(other) => panic!("expected a name clash, found {other:?}"),
            Ok(_) => panic!("expected a name clash"),
        }
    }

    fn clash(fields: Vec<Field>) -> NameClash {
        clash_in(&message_with(fields))
    }

    #[test]
    fn an_enum_named_like_a_message_yields() {
        let dictionary = message_with(vec![with_values(
            field("NewOrderSingle", 9000, DataType::Char),
            &[("1", "FIRST")],
        )]);
        let mut warnings = Vec::new();

        let names = Names::new(&dictionary, &mut warnings).unwrap();

        assert_eq!(
            names.enum_class(&dictionary.fields()[0]),
            "NewOrderSingleEnum"
        );
        assert_eq!(
            warnings,
            vec![Warning::Renamed {
                language: Language::Python,
                clash: Box::new(NameClash {
                    name: "NewOrderSingle".to_owned(),
                    first: Owner::Message("NewOrderSingle".to_owned()),
                    second: Owner::Enum("NewOrderSingle".to_owned()),
                }),
                name: "NewOrderSingleEnum".to_owned(),
            }]
        );
    }

    #[test]
    fn an_enum_named_like_an_import_clashes() {
        let field = with_values(field("RawMessage", 9000, DataType::Char), &[("1", "FIRST")]);
        assert_eq!(clash(vec![field]).first, Owner::Generated);
    }

    #[test]
    fn an_enum_named_like_a_shared_group_yields() {
        let dictionary = dictionary(
            vec![message(
                "MarketDataRequest",
                "V",
                vec![Member::Component(ComponentRef {
                    name: "RateSource".to_owned(),
                    is_required: false,
                })],
            )],
            vec![
                field("NoRateSources", 1445, DataType::NumInGroup),
                with_values(
                    field("RateSource", 1446, DataType::Int),
                    &[("0", "BLOOMBERG")],
                ),
            ],
            vec![Component {
                name: "RateSource".to_owned(),
                members: vec![group(1445, vec![field_ref(1446)])],
            }],
        );
        let mut warnings = Vec::new();

        let names = Names::new(&dictionary, &mut warnings).unwrap();

        assert_eq!(names.enum_class(&dictionary.fields()[1]), "RateSourceEnum");
        assert_eq!(
            warnings,
            vec![Warning::Renamed {
                language: Language::Python,
                clash: Box::new(NameClash {
                    name: "RateSource".to_owned(),
                    first: Owner::Group(
                        MemberContext::Component("RateSource".to_owned()).group("NoRateSources")
                    ),
                    second: Owner::Enum("RateSource".to_owned()),
                }),
                name: "RateSourceEnum".to_owned(),
            }]
        );
    }

    #[test]
    fn messages_may_nest_groups_of_one_name() {
        let routing = || group(215, vec![field_ref(216)]);
        let dictionary = dictionary(
            vec![
                message("IOI", "6", vec![routing()]),
                message("News", "B", vec![routing()]),
            ],
            vec![
                field("NoRoutingIDs", 215, DataType::NumInGroup),
                field("RoutingType", 216, DataType::Int),
            ],
            vec![],
        );
        assert!(Names::new(&dictionary, &mut Vec::new()).is_ok());
    }

    #[test]
    fn two_members_named_alike_are_numbered() {
        let dictionary = message_with(vec![with_values(
            field("BenchmarkCurveName", 221, DataType::String),
            &[("Euribor", "EURIBOR"), ("EURIBOR", "EURIBOR")],
        )]);
        let mut warnings = Vec::new();

        let names = Names::new(&dictionary, &mut warnings).unwrap();

        assert_eq!(
            names.enum_members(&dictionary.fields()[0]),
            ["EURIBOR", "EURIBOR_2"]
        );
        assert!(matches!(
            warnings.as_slice(),
            [Warning::Renamed {
                language: Language::Python,
                ..
            }]
        ));
    }

    #[test]
    fn two_fields_with_one_property_name_clash() {
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
    fn a_property_named_like_a_generated_member_clashes() {
        assert_eq!(
            clash(vec![field("Raw", 9000, DataType::String)]).first,
            Owner::Generated
        );
    }

    #[test]
    fn a_property_named_like_a_decorator_clashes() {
        assert_eq!(
            clash(vec![field("CachedProperty", 9000, DataType::String)]),
            NameClash {
                name: "cached_property".to_owned(),
                first: Owner::Generated,
                second: Owner::Field("CachedProperty".to_owned()),
            }
        );
    }

    #[test]
    fn a_clash_names_the_language() {
        let field = with_values(field("RawMessage", 9000, DataType::Char), &[("1", "FIRST")]);
        let error = Names::new(&message_with(vec![field]), &mut Vec::new())
            .err()
            .unwrap();
        assert_eq!(
            error.to_string(),
            "python name `RawMessage` is taken by both the generated code and the enum of field \
             'RawMessage'"
        );
    }
}
