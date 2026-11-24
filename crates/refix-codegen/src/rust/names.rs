use std::collections::HashMap;

use refix_dictionary::spec::Field;
use refix_dictionary::{Dictionary, MemberContext, Tag, dictionary};

use super::Error;
use super::naming::{
    component_accessor_name, group_accessor_name, group_type_name, method_name, numbered_variant,
    variant_name,
};
use crate::Warning;
use crate::components::components;
use crate::conversion::Conversion;
use crate::envelope::Section;
use crate::groups::{declared_groups, instance_context};
use crate::namespace::{Namespace, Owner};

/// Names the generated root defines or relies on.
const ROOT: &[&str] = &[
    "DIALECT",
    "Date",
    "Decimal",
    "Dialect",
    "From",
    "GroupTable",
    "Header",
    "Instances",
    "InvalidValue",
    "KnownTags",
    "Message",
    "MonthYear",
    "MultipleValues",
    "Ok",
    "Option",
    "RawMessage",
    "Result",
    "Some",
    "Tag",
    "Time",
    "Trailer",
    "UtcTimestamp",
    "bool",
    "i64",
    "refix_message",
    "std",
    "str",
    "u8",
];

/// The constants and methods every message defines.
const MESSAGE: &[&str] = &[
    "KNOWN_TAGS",
    "MSG_TYPE",
    "from_raw",
    "header",
    "raw",
    "trailer",
];

/// The methods every group instance and header or trailer view defines.
const INSTANCE: &[&str] = &["raw"];

/// The constant and method every component's view defines.
const COMPONENT: &[&str] = &["TAGS", "raw"];

/// Every name the generated Rust uses, settled before anything is emitted.
pub(super) struct Names {
    enums: HashMap<Tag, EnumNames>,
    accessors: HashMap<Tag, String>,
    group_types: HashMap<MemberContext, String>,
    group_accessors: HashMap<MemberContext, String>,
    component_accessors: HashMap<String, String>,
}

struct EnumNames {
    type_name: String,
    variants: Vec<String>,
}

impl Names {
    /// Names everything generated from `dictionary`.
    ///
    /// A clash that a rule settles is renamed with a warning. Any other
    /// clash is an error.
    pub(super) fn new(dictionary: &Dictionary, warnings: &mut Vec<Warning>) -> Result<Self, Error> {
        let mut names = Names {
            enums: HashMap::new(),
            accessors: HashMap::new(),
            group_types: HashMap::new(),
            group_accessors: HashMap::new(),
            component_accessors: HashMap::new(),
        };
        let mut root = Namespace::with_generated(ROOT);
        let mut message_variants = Namespace::with_generated(&["Unrecognized"]);
        for message in dictionary.messages() {
            root.claim(message.name(), Owner::Message(message.name().to_owned()))?;
            message_variants.claim(message.name(), Owner::Message(message.name().to_owned()))?;
        }
        for group in declared_groups(dictionary) {
            names.name_group_type(group, &mut root)?;
        }
        for component in components(dictionary) {
            let name = component.name();
            root.claim(name, Owner::Component(name.to_owned()))?;
            names
                .component_accessors
                .insert(name.to_owned(), component_accessor_name(name)?);
        }
        for section in [Section::Header, Section::Trailer] {
            names.name_accessors(
                section.listed(dictionary).into_iter(),
                Namespace::with_generated(INSTANCE),
            )?;
        }
        for field in dictionary.fields() {
            if Conversion::of(field).is_enum() {
                let type_name = root.claim_enum(&field.name, warnings)?;
                names.name_enum(field, type_name, warnings)?;
            }
        }
        for message in dictionary.messages() {
            names.name_accessors(message.members(), Namespace::with_generated(MESSAGE))?;
        }
        for group in declared_groups(dictionary) {
            names.name_accessors(group.members(), Namespace::with_generated(INSTANCE))?;
        }
        for component in components(dictionary) {
            names.name_accessors(component.members(), Namespace::with_generated(COMPONENT))?;
        }
        Ok(names)
    }

    /// Names the type of a group's instances.
    ///
    /// A group declared in another group's instances follows that group's
    /// type, which is named first.
    fn name_group_type(
        &mut self,
        group: dictionary::Group<'_>,
        root: &mut Namespace,
    ) -> Result<(), Error> {
        let owner = match group.declared_in() {
            MemberContext::Message(name) | MemberContext::Component(name) => name,
            MemberContext::Header => Section::Header.type_name(),
            MemberContext::Trailer => Section::Trailer.type_name(),
            outer @ MemberContext::Group { .. } => &self.group_types[outer],
        };
        let name = group_type_name(group, owner)?;
        root.claim(&name, Owner::Group(instance_context(group)))?;
        self.group_types.insert(instance_context(group), name);
        Ok(())
    }

    fn name_enum(
        &mut self,
        field: &Field,
        type_name: String,
        warnings: &mut Vec<Warning>,
    ) -> Result<(), Error> {
        let mut variants = Namespace::with_generated(&["Unrecognized"]);
        let names = field
            .values
            .iter()
            .map(|value| {
                let name = variant_name(field, value)?;
                let owner = Owner::Value {
                    field: field.name.clone(),
                    code: value.value.clone(),
                    description: value.description.clone(),
                };
                Ok(variants.claim_value(&name, owner, numbered_variant, warnings)?)
            })
            .collect::<Result<Vec<_>, Error>>()?;
        self.enums.insert(
            field.tag,
            EnumNames {
                type_name,
                variants: names,
            },
        );
        Ok(())
    }

    /// Names the accessors of a message, group instance or view.
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
                dictionary::Member::Group(group) => {
                    let name = group_accessor_name(group)?;
                    namespace.claim(&name, Owner::Group(instance_context(group)))?;
                    self.group_accessors.insert(instance_context(group), name);
                }
                dictionary::Member::Component(component) => {
                    let name = &self.component_accessors[component.name()];
                    namespace.claim(name, Owner::Component(component.name().to_owned()))?;
                }
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

    /// The type of a group's instances.
    pub(super) fn group_type(&self, group: dictionary::Group<'_>) -> &str {
        &self.group_types[&instance_context(group)]
    }

    /// The method reading a group's instances.
    pub(super) fn group_accessor(&self, group: dictionary::Group<'_>) -> &str {
        &self.group_accessors[&instance_context(group)]
    }

    /// The method returning a component's view.
    pub(super) fn component_accessor(&self, component: dictionary::Component<'_>) -> &str {
        &self.component_accessors[component.name()]
    }
}

#[cfg(test)]
mod tests {
    use super::Names;
    use crate::components::components;
    use crate::groups::declared_groups;
    use crate::rust::Error;
    use crate::test_utils::message_with;
    use crate::{NameClash, Owner, Warning};
    use refix_dictionary::spec::{
        Category, Component, ComponentRef, DataType, EnumValue, Field, FieldRef, Group, Member,
        Message, Protocol, Spec, Version,
    };
    use refix_dictionary::{Dictionary, MemberContext, Tag};

    fn field(name: &str, tag: u32, data_type: DataType) -> Field {
        Field {
            name: name.to_owned(),
            tag: Tag(tag),
            data_type,
            length_tag: None,
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

    /// A dictionary whose `Parties` component wraps the `NoPartyIDs` group.
    fn with_parties(messages: Vec<Message>, mut fields: Vec<Field>) -> Dictionary {
        fields.extend([
            field("NoPartyIDs", 453, DataType::NumInGroup),
            field("PartyID", 448, DataType::String),
        ]);
        Spec {
            version: Version {
                protocol: Protocol::Fix,
                major: 4,
                minor: 4,
                service_pack: 0,
            },
            header: vec![],
            trailer: vec![],
            messages,
            fields,
            components: vec![Component {
                name: "Parties".to_owned(),
                members: vec![group(453, vec![field_ref(448)])],
            }],
        }
        .resolve()
        .unwrap()
        .dictionary
    }

    fn parties_ref() -> Member {
        Member::Component(ComponentRef {
            name: "Parties".to_owned(),
            is_required: false,
        })
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
            names.enum_type(&dictionary.fields()[0]),
            "NewOrderSingleEnum"
        );
        assert_eq!(
            warnings,
            vec![Warning::Renamed {
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
    fn two_values_named_alike_are_numbered() {
        let dictionary = message_with(vec![with_values(
            field("BenchmarkCurveName", 221, DataType::String),
            &[("Euribor", "EURIBOR"), ("EURIBOR", "EURIBOR")],
        )]);
        let mut warnings = Vec::new();

        let names = Names::new(&dictionary, &mut warnings).unwrap();

        assert_eq!(
            names.variants(&dictionary.fields()[0]),
            ["Euribor", "Euribor2"]
        );
        assert!(matches!(warnings.as_slice(), [Warning::Renamed { .. }]));
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
    fn a_message_named_like_the_catch_all_clashes() {
        let dictionary = Spec {
            version: Version {
                protocol: Protocol::Fix,
                major: 4,
                minor: 4,
                service_pack: 0,
            },
            header: vec![],
            trailer: vec![],
            messages: vec![Message {
                name: "Unrecognized".to_owned(),
                msg_type: "U1".to_owned(),
                members: vec![],
                category: Category::App,
            }],
            fields: vec![],
            components: vec![],
        }
        .resolve()
        .unwrap()
        .dictionary;

        let Err(Error::NameClash(clash)) = Names::new(&dictionary, &mut Vec::new()) else {
            panic!("expected a name clash");
        };
        assert_eq!(
            *clash,
            NameClash {
                name: "Unrecognized".to_owned(),
                first: Owner::Generated,
                second: Owner::Message("Unrecognized".to_owned()),
            }
        );
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
    fn an_accessor_named_like_the_header_view_clashes() {
        assert_eq!(
            clash(vec![field("Header", 9000, DataType::String)]),
            NameClash {
                name: "header".to_owned(),
                first: Owner::Generated,
                second: Owner::Field("Header".to_owned()),
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
    fn a_group_accessor_named_like_a_field_accessor_clashes() {
        let dictionary = with_parties(
            vec![message(
                "NewOrderSingle",
                "D",
                vec![field_ref(9000), parties_ref()],
            )],
            vec![field("PartyIDs", 9000, DataType::String)],
        );
        assert_eq!(
            clash_in(&dictionary),
            NameClash {
                name: "party_ids".to_owned(),
                first: Owner::Field("PartyIDs".to_owned()),
                second: Owner::Group(
                    MemberContext::Component("Parties".to_owned()).group("NoPartyIDs")
                ),
            }
        );
    }

    /// A dictionary whose `component` holds `Symbol`.
    fn with_component(
        component: &str,
        messages: Vec<Message>,
        mut fields: Vec<Field>,
    ) -> Dictionary {
        fields.push(field("Symbol", 55, DataType::String));
        Spec {
            version: Version {
                protocol: Protocol::Fix,
                major: 4,
                minor: 4,
                service_pack: 0,
            },
            header: vec![],
            trailer: vec![],
            messages,
            fields,
            components: vec![Component {
                name: component.to_owned(),
                members: vec![field_ref(55)],
            }],
        }
        .resolve()
        .unwrap()
        .dictionary
    }

    fn component_ref(name: &str) -> Member {
        Member::Component(ComponentRef {
            name: name.to_owned(),
            is_required: false,
        })
    }

    fn order_with(members: Vec<Member>) -> Vec<Message> {
        vec![message("NewOrderSingle", "D", members)]
    }

    #[test]
    fn a_view_accessor_is_the_component_in_snake_case() {
        let messages = order_with(vec![component_ref("OrderQtyData")]);
        let dictionary = with_component("OrderQtyData", messages, vec![]);
        let names = Names::new(&dictionary, &mut Vec::new()).unwrap();
        let order_qty_data = components(&dictionary)[0];

        assert_eq!(names.component_accessor(order_qty_data), "order_qty_data");
    }

    #[test]
    fn a_view_accessor_named_like_a_field_accessor_clashes() {
        let messages = order_with(vec![field_ref(9000), component_ref("Instrument")]);
        let dictionary = with_component(
            "Instrument",
            messages,
            vec![field("Instrument", 9000, DataType::String)],
        );

        assert_eq!(
            clash_in(&dictionary),
            NameClash {
                name: "instrument".to_owned(),
                first: Owner::Field("Instrument".to_owned()),
                second: Owner::Component("Instrument".to_owned()),
            }
        );
    }

    #[test]
    fn a_component_may_hold_a_field_of_its_own_name() {
        let messages = order_with(vec![component_ref("Symbol")]);
        let dictionary = with_component("Symbol", messages, vec![]);
        let names = Names::new(&dictionary, &mut Vec::new()).unwrap();
        let symbol = components(&dictionary)[0];

        assert_eq!(names.component_accessor(symbol), "symbol");
        assert_eq!(names.accessor(&dictionary.fields()[0]), "symbol");
    }

    #[test]
    fn a_component_named_like_a_message_clashes() {
        let messages = vec![message("Quote", "S", vec![component_ref("Quote")])];
        let dictionary = with_component("Quote", messages, vec![]);

        assert_eq!(
            clash_in(&dictionary),
            NameClash {
                name: "Quote".to_owned(),
                first: Owner::Message("Quote".to_owned()),
                second: Owner::Component("Quote".to_owned()),
            }
        );
    }

    #[test]
    fn an_enum_named_like_a_component_yields() {
        let messages = order_with(vec![component_ref("Instrument")]);
        let dictionary = with_component(
            "Instrument",
            messages,
            vec![with_values(
                field("Instrument", 9000, DataType::Char),
                &[("1", "FIRST")],
            )],
        );
        let mut warnings = Vec::new();

        let names = Names::new(&dictionary, &mut warnings).unwrap();

        assert_eq!(names.enum_type(&dictionary.fields()[0]), "InstrumentEnum");
        assert_eq!(
            warnings,
            vec![Warning::Renamed {
                clash: Box::new(NameClash {
                    name: "Instrument".to_owned(),
                    first: Owner::Component("Instrument".to_owned()),
                    second: Owner::Enum("Instrument".to_owned()),
                }),
                name: "InstrumentEnum".to_owned(),
            }]
        );
    }

    #[test]
    fn a_group_type_named_like_a_message_clashes() {
        let dictionary = with_parties(
            vec![
                message("Parties", "X", vec![]),
                message("NewOrderSingle", "D", vec![parties_ref()]),
            ],
            vec![],
        );
        assert_eq!(
            clash_in(&dictionary),
            NameClash {
                name: "Parties".to_owned(),
                first: Owner::Message("Parties".to_owned()),
                second: Owner::Group(
                    MemberContext::Component("Parties".to_owned()).group("NoPartyIDs")
                ),
            }
        );
    }

    #[test]
    fn a_group_in_a_group_instance_follows_its_parents_type() {
        let dictionary = with_parties(
            vec![message(
                "Logon",
                "A",
                vec![group(
                    384,
                    vec![field_ref(372), group(9001, vec![field_ref(9002)])],
                )],
            )],
            vec![
                field("NoMsgTypes", 384, DataType::NumInGroup),
                field("RefMsgType", 372, DataType::String),
                field("NoMsgSubTypes", 9001, DataType::NumInGroup),
                field("MsgSubType", 9002, DataType::String),
            ],
        );
        let names = Names::new(&dictionary, &mut Vec::new()).unwrap();
        let groups = declared_groups(&dictionary);

        assert_eq!(names.group_type(groups[0]), "LogonMsgTypes");
        assert_eq!(names.group_type(groups[1]), "LogonMsgTypesMsgSubTypes");
    }

    #[test]
    fn a_clash_names_the_language() {
        let field = with_values(field("Result", 9000, DataType::Char), &[("1", "FIRST")]);
        let error = Names::new(&message_with(vec![field]), &mut Vec::new())
            .err()
            .unwrap();
        assert_eq!(
            error.to_string(),
            "rust name `Result` is taken by both the generated code and the enum of field 'Result'"
        );
    }
}
