use crate::{
    Category, Component, ComponentRef, DataType, Dictionary, EnumValue, Field, FieldRef, Group,
    Member, MemberContext, Message, Protocol, Spec, Tag, Version, dictionary,
};
use roxmltree::Node;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::num::ParseIntError;
use std::str::FromStr;

/// Parses a QuickFIX-format XML data dictionary into a [`Dictionary`].
pub fn parse(xml: &str) -> Result<Parsed, Error> {
    let document = roxmltree::Document::parse(xml).map_err(Error::Xml)?;
    let root = document.root_element();
    if !root.has_tag_name("fix") {
        return Err(Error::UnexpectedRoot(root.tag_name().name().to_owned()));
    }

    let mut warnings = Vec::new();
    for section in ["header", "trailer"] {
        if root.children().any(|node| node.has_tag_name(section)) {
            warnings.push(Warning::UnsupportedSection {
                section: section.to_owned(),
            });
        }
    }

    let version = parse_version(root)?;
    let fields = parse_fields(root, &mut warnings)?;
    let tags_by_name: HashMap<&str, Tag> = fields
        .iter()
        .map(|field| (field.name.as_str(), field.tag))
        .collect();

    let components = parse_components(root, &tags_by_name, &mut warnings)?;
    let messages = parse_messages(root, &tags_by_name, &mut warnings)?;

    let spec = Spec {
        version,
        header: Vec::new(),
        trailer: Vec::new(),
        messages,
        components,
        fields,
    };
    let dictionary = spec.resolve().map_err(Error::Invalid)?;

    Ok(Parsed {
        dictionary,
        warnings,
    })
}

fn parse_version(root: Node) -> Result<Version, Error> {
    let protocol = match root.attribute("type") {
        Some("FIX") | None => Protocol::Fix,
        Some("FIXT") => Protocol::Fixt,
        Some(other) => return Err(Error::UnknownProtocol(other.to_owned())),
    };

    Ok(Version {
        protocol,
        major: int_attribute(root, "major")?,
        minor: int_attribute(root, "minor")?,
        service_pack: int_attribute_or(root, "servicepack", 0)?,
    })
}

fn parse_fields(root: Node, warnings: &mut Vec<Warning>) -> Result<Vec<Field>, Error> {
    let Some(section) = root.children().find(|node| node.has_tag_name("fields")) else {
        return Ok(Vec::new());
    };

    let fields: Vec<Field> = section
        .children()
        .filter(|node| node.has_tag_name("field"))
        .map(|node| parse_field(node, warnings))
        .collect::<Result<_, _>>()?;

    let mut seen = HashSet::new();
    for field in &fields {
        if !seen.insert(field.name.as_str()) {
            return Err(Error::DuplicateField {
                field: field.name.clone(),
            });
        }
    }

    Ok(fields)
}

fn parse_field(node: Node, warnings: &mut Vec<Warning>) -> Result<Field, Error> {
    let name = string_attribute(node, "name")?;
    let values = node
        .children()
        .filter(|node| node.has_tag_name("value"))
        .map(|value| {
            Ok(EnumValue {
                value: string_attribute(value, "enum")?,
                description: string_attribute(value, "description")?,
            })
        })
        .collect::<Result<_, _>>()?;
    let tag = Tag(int_attribute(node, "number")?);
    let data_type = parse_data_type(string_attribute(node, "type")?);
    let values = drop_repeated_codes(&name, &data_type, values, warnings);

    Ok(Field {
        name,
        tag,
        data_type,
        values,
    })
}

/// Keeps the first value listed for each code, warning of the rest.
fn drop_repeated_codes(
    field: &str,
    data_type: &DataType,
    values: Vec<EnumValue>,
    warnings: &mut Vec<Warning>,
) -> Vec<EnumValue> {
    let mut kept: Vec<EnumValue> = Vec::new();
    let mut by_code: HashMap<String, usize> = HashMap::new();
    for value in values {
        match by_code.entry(data_type.canonical_code(&value.value).into_owned()) {
            Entry::Occupied(first) => warnings.push(Warning::RepeatedCode {
                field: field.to_owned(),
                kept: kept[*first.get()].clone(),
                dropped: value,
            }),
            Entry::Vacant(free) => {
                free.insert(kept.len());
                kept.push(value);
            }
        }
    }
    kept
}

fn parse_components(
    root: Node,
    tags_by_name: &HashMap<&str, Tag>,
    warnings: &mut Vec<Warning>,
) -> Result<Vec<Component>, Error> {
    let Some(section) = root.children().find(|node| node.has_tag_name("components")) else {
        return Ok(Vec::new());
    };

    let components: Vec<Component> = section
        .children()
        .filter(|node| node.has_tag_name("component"))
        .map(|node| parse_component(node, tags_by_name, warnings))
        .collect::<Result<_, _>>()?;

    Ok(components)
}

fn parse_component(
    node: Node,
    tags_by_name: &HashMap<&str, Tag>,
    warnings: &mut Vec<Warning>,
) -> Result<Component, Error> {
    let name = string_attribute(node, "name")?;
    let context = MemberContext::Component(name.clone());
    let members = parse_members(node, context, tags_by_name, warnings)?;

    Ok(Component { name, members })
}

fn parse_data_type(name: String) -> DataType {
    match name.as_str() {
        "STRING" => DataType::String,
        "INT" => DataType::Int,
        "LENGTH" => DataType::Length,
        "NUMINGROUP" => DataType::NumInGroup,
        "SEQNUM" => DataType::SeqNum,
        "TAGNUM" => DataType::TagNum,
        "DAYOFMONTH" => DataType::DayOfMonth,
        "CHAR" => DataType::Char,
        "MULTIPLECHARVALUE" => DataType::MultipleCharValue,
        "MULTIPLESTRINGVALUE" | "MULTIPLEVALUESTRING" => DataType::MultipleStringValue,
        _ => DataType::Other(name),
    }
}

fn parse_messages(
    root: Node,
    tags_by_name: &HashMap<&str, Tag>,
    warnings: &mut Vec<Warning>,
) -> Result<Vec<Message>, Error> {
    let Some(section) = root.children().find(|node| node.has_tag_name("messages")) else {
        return Ok(Vec::new());
    };

    section
        .children()
        .filter(|node| node.has_tag_name("message"))
        .map(|node| parse_message(node, tags_by_name, warnings))
        .collect()
}

fn parse_message(
    node: Node,
    tags_by_name: &HashMap<&str, Tag>,
    warnings: &mut Vec<Warning>,
) -> Result<Message, Error> {
    let name = string_attribute(node, "name")?;
    let category = parse_category(node)?;
    let members = parse_members(
        node,
        MemberContext::Message(name.clone()),
        tags_by_name,
        warnings,
    )?;

    Ok(Message {
        name,
        msg_type: string_attribute(node, "msgtype")?,
        members,
        category,
    })
}

fn parse_members(
    node: Node,
    context: MemberContext,
    tags_by_name: &HashMap<&str, Tag>,
    warnings: &mut Vec<Warning>,
) -> Result<Vec<Member>, Error> {
    let mut members = Vec::new();

    for child in node.children().filter(Node::is_element) {
        match child.tag_name().name() {
            "field" => members.push(Member::Field(parse_field_ref(
                child,
                context.clone(),
                tags_by_name,
            )?)),
            "component" => members.push(Member::Component(parse_component_ref(child)?)),
            "group" => members.push(Member::Group(parse_group(
                child,
                &context,
                tags_by_name,
                warnings,
            )?)),
            other => warnings.push(Warning::UnsupportedElement {
                context: context.clone(),
                element: other.to_owned(),
            }),
        }
    }

    Ok(members)
}

fn parse_field_ref(
    node: Node,
    context: MemberContext,
    tags_by_name: &HashMap<&str, Tag>,
) -> Result<FieldRef, Error> {
    let name = string_attribute(node, "name")?;
    let Some(&tag) = tags_by_name.get(name.as_str()) else {
        return Err(Error::UnknownField {
            context,
            field: name,
        });
    };

    Ok(FieldRef {
        tag,
        is_required: required_attribute(node)?,
    })
}

fn parse_component_ref(node: Node) -> Result<ComponentRef, Error> {
    let name = string_attribute(node, "name")?;

    Ok(ComponentRef {
        name,
        is_required: required_attribute(node)?,
    })
}

fn parse_group(
    node: Node,
    context: &MemberContext,
    tags_by_name: &HashMap<&str, Tag>,
    warnings: &mut Vec<Warning>,
) -> Result<Group, Error> {
    // A group element names and flags its count field the way a field
    // reference does.
    let count = parse_field_ref(node, context.clone(), tags_by_name)?;
    let name = string_attribute(node, "name")?;
    let members = parse_members(node, context.group(&name), tags_by_name, warnings)?;

    Ok(Group {
        count_tag: count.tag,
        is_required: count.is_required,
        members,
    })
}

fn parse_category(node: Node) -> Result<Category, Error> {
    match node.attribute("msgcat") {
        Some("admin") => Ok(Category::Admin),
        Some("app") => Ok(Category::App),
        Some(other) => Err(Error::InvalidAttribute {
            element: node.tag_name().name().to_owned(),
            attribute: "msgcat".to_owned(),
            value: other.to_owned(),
        }),
        None => Err(Error::MissingAttribute {
            element: node.tag_name().name().to_owned(),
            attribute: "msgcat".to_owned(),
        }),
    }
}

fn required_attribute(node: Node) -> Result<bool, Error> {
    match node.attribute("required") {
        Some("Y") => Ok(true),
        Some("N") | None => Ok(false),
        Some(other) => Err(Error::InvalidAttribute {
            element: node.tag_name().name().to_owned(),
            attribute: "required".to_owned(),
            value: other.to_owned(),
        }),
    }
}

fn string_attribute(node: Node, name: &str) -> Result<String, Error> {
    node.attribute(name)
        .map(str::to_owned)
        .ok_or_else(|| Error::MissingAttribute {
            element: node.tag_name().name().to_owned(),
            attribute: name.to_owned(),
        })
}

fn int_attribute<T: FromStr<Err = ParseIntError>>(node: Node, name: &str) -> Result<T, Error> {
    match node.attribute(name) {
        Some(value) => T::from_str(value).map_err(|_| Error::InvalidNumber {
            element: node.tag_name().name().to_owned(),
            attribute: name.to_owned(),
            value: value.to_owned(),
        }),
        None => Err(Error::MissingAttribute {
            element: node.tag_name().name().to_owned(),
            attribute: name.to_owned(),
        }),
    }
}

fn int_attribute_or<T: FromStr<Err = ParseIntError>>(
    node: Node,
    name: &str,
    default: T,
) -> Result<T, Error> {
    match node.attribute(name) {
        Some(_) => int_attribute(node, name),
        None => Ok(default),
    }
}

/// The result of a successful parse.
///
/// This contains the resolved dictionary as well as any
/// [`Warning`] that was produced along the way.
#[derive(Debug)]
pub struct Parsed {
    pub dictionary: Dictionary,
    pub warnings: Vec<Warning>,
}

/// Something the parser read but left out of the dictionary.
#[derive(Debug, Eq, PartialEq)]
pub enum Warning {
    UnsupportedElement {
        context: MemberContext,
        element: String,
    },
    UnsupportedSection {
        section: String,
    },
    /// A value listed under the code of an earlier value of its field.
    RepeatedCode {
        field: String,
        kept: EnumValue,
        dropped: EnumValue,
    },
}

impl fmt::Display for Warning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Warning::UnsupportedElement { context, element } => {
                write!(f, "unexpected element <{element}> in {context}")
            }
            Warning::UnsupportedSection { section } => {
                write!(f, "section <{section}> is not supported yet")
            }
            Warning::RepeatedCode {
                field,
                kept,
                dropped,
            } => write!(
                f,
                "value '{}' (code {}) of field '{field}' is dropped, as value '{}' (code {}) has \
                 the same code",
                dropped.description, dropped.value, kept.description, kept.value
            ),
        }
    }
}

/// A defect that prevents producing a dictionary at all.
///
/// Anything survivable is a [`Warning`] instead; see [`parse`].
#[derive(Debug)]
pub enum Error {
    Xml(roxmltree::Error),
    UnexpectedRoot(String),
    UnknownProtocol(String),
    MissingAttribute {
        element: String,
        attribute: String,
    },
    InvalidNumber {
        element: String,
        attribute: String,
        value: String,
    },
    UnknownField {
        context: MemberContext,
        field: String,
    },
    DuplicateField {
        field: String,
    },
    InvalidAttribute {
        element: String,
        attribute: String,
        value: String,
    },
    Invalid(dictionary::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Xml(error) => write!(f, "malformed xml: {error}"),
            Error::UnexpectedRoot(root) => {
                write!(f, "unexpected root element <{root}>, expected <fix>")
            }
            Error::UnknownProtocol(protocol) => {
                write!(
                    f,
                    "unknown protocol type '{protocol}', expected FIX or FIXT"
                )
            }
            Error::MissingAttribute { element, attribute } => {
                write!(f, "missing attribute '{attribute}' on <{element}>")
            }
            Error::InvalidNumber {
                element,
                attribute,
                value,
            } => {
                write!(
                    f,
                    "invalid number '{value}' in attribute '{attribute}' of <{element}>"
                )
            }
            Error::UnknownField { context, field } => {
                write!(f, "{context} references unknown field '{field}'")
            }
            Error::DuplicateField { field } => {
                write!(f, "field '{field}' is defined more than once")
            }
            Error::InvalidAttribute {
                element,
                attribute,
                value,
            } => {
                write!(
                    f,
                    "invalid value '{value}' in attribute '{attribute}' of <{element}>"
                )
            }
            Error::Invalid(error) => {
                write!(f, "invalid dictionary: {error}")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Xml(error) => Some(error),
            Error::Invalid(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version_of(xml: &str) -> Version {
        parse(xml).unwrap().dictionary.version()
    }

    mod version {
        use super::*;

        #[test]
        fn full_version_attributes() {
            let version = version_of("<fix type='FIX' major='4' minor='4' servicepack='2'/>");
            assert_eq!(
                version,
                Version {
                    protocol: Protocol::Fix,
                    major: 4,
                    minor: 4,
                    service_pack: 2,
                }
            );
        }

        #[test]
        fn fixt_protocol() {
            let version = version_of("<fix type='FIXT' major='1' minor='1' servicepack='0'/>");
            assert_eq!(version.protocol, Protocol::Fixt);
        }

        #[test]
        fn missing_type_defaults_to_fix() {
            let version = version_of("<fix major='4' minor='2' servicepack='0'/>");
            assert_eq!(version.protocol, Protocol::Fix);
        }

        #[test]
        fn missing_servicepack_defaults_to_zero() {
            let version = version_of("<fix type='FIX' major='4' minor='4'/>");
            assert_eq!(version.service_pack, 0);
        }
    }

    mod fields {
        use super::*;

        const DICTIONARY: &str = "\
<fix major='4' minor='4'>
 <fields>
  <field number='11' name='ClOrdID' type='STRING'/>
  <field number='423' name='PriceType' type='INT'/>
  <field number='38' name='OrderQty' type='QTY'/>
  <field number='40' name='OrdType' type='CHAR'>
   <value enum='1' description='MARKET'/>
   <value enum='2' description='LIMIT'/>
  </field>
 </fields>
</fix>";

        #[test]
        fn parses_field_definitions() {
            let fields = parse(DICTIONARY).unwrap().dictionary.spec().fields.clone();

            assert_eq!(
                fields,
                vec![
                    Field {
                        name: "ClOrdID".to_owned(),
                        tag: Tag(11),
                        data_type: DataType::String,
                        values: vec![],
                    },
                    Field {
                        name: "PriceType".to_owned(),
                        tag: Tag(423),
                        data_type: DataType::Int,
                        values: vec![],
                    },
                    Field {
                        name: "OrderQty".to_owned(),
                        tag: Tag(38),
                        data_type: DataType::Other("QTY".to_owned()),
                        values: vec![],
                    },
                    Field {
                        name: "OrdType".to_owned(),
                        tag: Tag(40),
                        data_type: DataType::Char,
                        values: vec![
                            EnumValue {
                                value: "1".to_owned(),
                                description: "MARKET".to_owned(),
                            },
                            EnumValue {
                                value: "2".to_owned(),
                                description: "LIMIT".to_owned(),
                            },
                        ],
                    },
                ]
            );
        }

        #[test]
        fn maps_type_names_to_data_types() {
            let cases = [
                ("STRING", DataType::String),
                ("INT", DataType::Int),
                ("LENGTH", DataType::Length),
                ("NUMINGROUP", DataType::NumInGroup),
                ("SEQNUM", DataType::SeqNum),
                ("TAGNUM", DataType::TagNum),
                ("DAYOFMONTH", DataType::DayOfMonth),
                ("CHAR", DataType::Char),
                ("MULTIPLECHARVALUE", DataType::MultipleCharValue),
                ("MULTIPLESTRINGVALUE", DataType::MultipleStringValue),
                ("MULTIPLEVALUESTRING", DataType::MultipleStringValue),
                ("PRICE", DataType::Other("PRICE".to_owned())),
            ];
            for (name, data_type) in cases {
                assert_eq!(parse_data_type(name.to_owned()), data_type, "{name}");
            }
        }

        #[test]
        fn value_without_enum_is_an_error() {
            let error = parse(
                "<fix major='4' minor='4'><fields>\
                 <field number='40' name='OrdType' type='CHAR'>\
                 <value description='MARKET'/>\
                 </field></fields></fix>",
            )
            .unwrap_err();
            assert!(matches!(
                error,
                Error::MissingAttribute { ref element, ref attribute }
                    if element == "value" && attribute == "enum"
            ));
        }

        #[test]
        fn value_without_description_is_an_error() {
            let error = parse(
                "<fix major='4' minor='4'><fields>\
                 <field number='40' name='OrdType' type='CHAR'>\
                 <value enum='1'/>\
                 </field></fields></fix>",
            )
            .unwrap_err();
            assert!(matches!(
                error,
                Error::MissingAttribute { ref element, ref attribute }
                    if element == "value" && attribute == "description"
            ));
        }

        fn value(code: &str, description: &str) -> EnumValue {
            EnumValue {
                value: code.to_owned(),
                description: description.to_owned(),
            }
        }

        #[test]
        fn a_repeated_code_keeps_its_first_value() {
            let parsed = parse(
                "<fix major='4' minor='3'><fields>\
                 <field number='574' name='MatchType' type='STRING'>\
                 <value enum='M1' description='EXACT_MATCH'/>\
                 <value enum='M2' description='SUMMARIZED_MATCH'/>\
                 <value enum='M1' description='ACT_M1_MATCH'/>\
                 </field></fields></fix>",
            )
            .unwrap();

            assert_eq!(
                parsed.dictionary.fields()[0].values,
                [value("M1", "EXACT_MATCH"), value("M2", "SUMMARIZED_MATCH")]
            );
            assert_eq!(
                parsed.warnings,
                [Warning::RepeatedCode {
                    field: "MatchType".to_owned(),
                    kept: value("M1", "EXACT_MATCH"),
                    dropped: value("M1", "ACT_M1_MATCH"),
                }]
            );
        }

        #[test]
        fn int_codes_repeat_by_number() {
            let parsed = parse(
                "<fix major='4' minor='4'><fields>\
                 <field number='423' name='PriceType' type='INT'>\
                 <value enum='1' description='PERCENTAGE'/>\
                 <value enum='01' description='PERCENT'/>\
                 </field></fields></fix>",
            )
            .unwrap();

            assert_eq!(
                parsed.dictionary.fields()[0].values,
                [value("1", "PERCENTAGE")]
            );
            assert_eq!(parsed.warnings.len(), 1);
        }

        #[test]
        fn a_repeated_code_names_both_values() {
            let warning = Warning::RepeatedCode {
                field: "PriceType".to_owned(),
                kept: value("1", "PERCENTAGE"),
                dropped: value("01", "PERCENT"),
            };
            assert_eq!(
                warning.to_string(),
                "value 'PERCENT' (code 01) of field 'PriceType' is dropped, as value 'PERCENTAGE' \
                 (code 1) has the same code"
            );
        }

        #[test]
        fn missing_fields_section_yields_no_fields() {
            let parsed = parse("<fix major='4' minor='4'/>").unwrap();
            assert!(parsed.dictionary.fields().is_empty());
        }

        #[test]
        fn duplicate_field_definition_is_an_error() {
            let error = parse(
                "<fix major='4' minor='4'><fields>\
                 <field number='11' name='ClOrdID' type='STRING'/>\
                 <field number='12' name='ClOrdID' type='STRING'/>\
                 </fields></fix>",
            )
            .unwrap_err();

            assert!(matches!(error, Error::DuplicateField { ref field } if field == "ClOrdID"));
        }

        #[test]
        fn field_without_number_is_an_error() {
            let error = parse(
                "<fix major='4' minor='4'><fields><field name='ClOrdID' type='STRING'/></fields></fix>",
            )
                .unwrap_err();
            assert!(matches!(
                error,
                Error::MissingAttribute { ref element, ref attribute }
                    if element == "field" && attribute == "number"
            ));
        }
    }

    mod components {
        use super::*;

        const DICTIONARY: &str = "\
<fix major='4' minor='4'>
 <fields>
  <field number='12' name='Commission' type='AMT'/>
  <field number='13' name='CommType' type='CHAR'/>
  <field number='539' name='NoNestedPartyIDs' type='NUMINGROUP'/>
  <field number='524' name='NestedPartyID' type='STRING'/>
 </fields>
 <components>
  <component name='CommissionData'>
   <field name='Commission' required='Y'/>
   <field name='CommType'/>
   <group name='NoNestedPartyIDs' required='N'>
    <field name='NestedPartyID' required='Y'/>
   </group>
  </component>
  <component name='SpreadOrBenchmarkCurveData'>
   <component name='CommissionData' required='Y'/>
  </component>
 </components>
</fix>";

        #[test]
        fn parses_component_definitions() {
            let components = parse(DICTIONARY)
                .unwrap()
                .dictionary
                .spec()
                .components
                .clone();

            assert_eq!(
                components,
                vec![
                    Component {
                        name: "CommissionData".to_owned(),
                        members: vec![
                            Member::Field(FieldRef {
                                tag: Tag(12),
                                is_required: true,
                            }),
                            Member::Field(FieldRef {
                                tag: Tag(13),
                                is_required: false,
                            }),
                            Member::Group(Group {
                                count_tag: Tag(539),
                                is_required: false,
                                members: vec![Member::Field(FieldRef {
                                    tag: Tag(524),
                                    is_required: true,
                                })],
                            }),
                        ],
                    },
                    Component {
                        name: "SpreadOrBenchmarkCurveData".to_owned(),
                        members: vec![Member::Component(ComponentRef {
                            name: "CommissionData".to_owned(),
                            is_required: true,
                        })],
                    },
                ]
            );
        }

        #[test]
        fn an_undefined_field_in_a_group_names_the_group() {
            let error = parse(
                "<fix major='4' minor='4'>\
                 <fields><field number='453' name='NoPartyIDs' type='NUMINGROUP'/></fields>\
                 <components><component name='Parties'>\
                 <group name='NoPartyIDs'><field name='PartyID'/></group>\
                 </component></components></fix>",
            )
            .unwrap_err();

            let parties = MemberContext::Component("Parties".to_owned());
            assert!(matches!(
                error,
                Error::UnknownField { context, ref field }
                    if context == parties.group("NoPartyIDs") && field == "PartyID"
            ));
        }

        #[test]
        fn missing_components_section_yields_no_components() {
            let parsed = parse("<fix major='4' minor='4'/>").unwrap();
            assert!(parsed.dictionary.spec().components.is_empty());
        }
    }

    mod messages {
        use super::*;

        const DICTIONARY: &str = "\
<fix major='4' minor='4'>
 <fields>
  <field number='11' name='ClOrdID' type='STRING'/>
  <field number='58' name='Text' type='STRING'/>
  <field number='448' name='PartyID' type='STRING'/>
  <field number='78' name='NoAllocs' type='NUMINGROUP'/>
  <field number='79' name='AllocAccount' type='STRING'/>
 </fields>
 <components>
  <component name='Parties'>
   <field name='PartyID'/>
  </component>
 </components>
 <messages>
  <message name='NewOrderSingle' msgtype='D' msgcat='app'>
   <field name='ClOrdID' required='Y'/>
   <field name='Text'/>
   <component name='Parties' required='N'/>
   <group name='NoAllocs' required='N'>
    <field name='AllocAccount'/>
   </group>
  </message>
 </messages>
</fix>";

        #[test]
        fn parses_message_definitions() {
            let messages = parse(DICTIONARY)
                .unwrap()
                .dictionary
                .spec()
                .messages
                .clone();

            assert_eq!(
                messages,
                vec![Message {
                    name: "NewOrderSingle".to_owned(),
                    msg_type: "D".to_owned(),
                    members: vec![
                        Member::Field(FieldRef {
                            tag: Tag(11),
                            is_required: true,
                        }),
                        Member::Field(FieldRef {
                            tag: Tag(58),
                            is_required: false,
                        }),
                        Member::Component(ComponentRef {
                            name: "Parties".to_owned(),
                            is_required: false,
                        }),
                        Member::Group(Group {
                            count_tag: Tag(78),
                            is_required: false,
                            members: vec![Member::Field(FieldRef {
                                tag: Tag(79),
                                is_required: false,
                            })],
                        }),
                    ],
                    category: Category::App,
                }]
            );
        }

        #[test]
        fn groups_parse_without_warnings() {
            let parsed = parse(DICTIONARY).unwrap();
            assert!(parsed.warnings.is_empty());
        }

        #[test]
        fn a_group_with_an_undefined_count_field_is_an_error() {
            let error = parse(
                "<fix major='4' minor='4'><messages>\
                 <message name='NewOrderSingle' msgtype='D' msgcat='app'>\
                 <group name='NoAllocs'/></message></messages></fix>",
            )
            .unwrap_err();

            assert!(matches!(
                error,
                Error::UnknownField { context, ref field }
                    if context == MemberContext::Message("NewOrderSingle".to_owned())
                        && field == "NoAllocs"
            ));
        }

        #[test]
        fn an_unknown_element_in_a_group_names_the_group() {
            let parsed = parse(
                "<fix major='4' minor='4'>\
                 <fields>\
                 <field number='78' name='NoAllocs' type='NUMINGROUP'/>\
                 <field number='79' name='AllocAccount' type='STRING'/>\
                 </fields>\
                 <messages><message name='NewOrderSingle' msgtype='D' msgcat='app'>\
                 <group name='NoAllocs'><field name='AllocAccount'/><bogus/></group>\
                 </message></messages></fix>",
            )
            .unwrap();

            let message = MemberContext::Message("NewOrderSingle".to_owned());
            assert_eq!(
                parsed.warnings,
                vec![Warning::UnsupportedElement {
                    context: message.group("NoAllocs"),
                    element: "bogus".to_owned(),
                }]
            );
        }

        #[test]
        fn unknown_message_child_surfaces_as_a_warning() {
            let parsed = parse(
                "<fix major='4' minor='4'><messages>\
                 <message name='Heartbeat' msgtype='0' msgcat='admin'><bogus/></message>\
                 </messages></fix>",
            )
            .unwrap();

            assert_eq!(
                parsed.warnings,
                vec![Warning::UnsupportedElement {
                    context: MemberContext::Message("Heartbeat".to_owned()),
                    element: "bogus".to_owned(),
                }]
            );
        }

        #[test]
        fn missing_messages_section_yields_no_messages() {
            let parsed = parse("<fix major='4' minor='4'/>").unwrap();
            assert!(parsed.dictionary.spec().messages.is_empty());
        }

        #[test]
        fn field_ref_to_undefined_field_is_an_error() {
            let error = parse(
                "<fix major='4' minor='4'><messages>\
                 <message name='Heartbeat' msgtype='0' msgcat='admin'>\
                 <field name='TestReqID'/></message></messages></fix>",
            )
            .unwrap_err();

            assert!(matches!(
                error,
                Error::UnknownField { context, ref field }
                    if context == MemberContext::Message("Heartbeat".to_owned()) && field == "TestReqID"
            ));
        }

        #[test]
        fn invalid_required_is_an_error() {
            let error = parse(
                "<fix major='4' minor='4'>\
                 <fields><field number='112' name='TestReqID' type='STRING'/></fields>\
                 <messages><message name='Heartbeat' msgtype='0' msgcat='admin'>\
                 <field name='TestReqID' required='X'/></message></messages></fix>",
            )
            .unwrap_err();

            assert!(matches!(
                error,
                Error::InvalidAttribute { ref attribute, ref value, .. }
                    if attribute == "required" && value == "X"
            ));
        }

        #[test]
        fn missing_msgcat_is_an_error() {
            let error = parse(
                "<fix major='4' minor='4'><messages>\
                 <message name='Heartbeat' msgtype='0'/></messages></fix>",
            )
            .unwrap_err();

            assert!(matches!(
                error,
                Error::MissingAttribute { ref element, ref attribute }
                    if element == "message" && attribute == "msgcat"
            ));
        }

        #[test]
        fn invalid_msgcat_is_an_error() {
            let error = parse(
                "<fix major='4' minor='4'><messages>\
                 <message name='Heartbeat' msgtype='0' msgcat='session'/></messages></fix>",
            )
            .unwrap_err();

            assert!(matches!(
                error,
                Error::InvalidAttribute { ref attribute, ref value, .. }
                    if attribute == "msgcat" && value == "session"
            ));
        }
    }

    mod errors {
        use super::*;

        #[test]
        fn an_inconsistent_spec_is_an_error() {
            let error = parse(
                "<fix major='4' minor='4'><messages>\
                 <message name='NewOrderSingle' msgtype='D' msgcat='app'>\
                 <component name='Parties'/>\
                 </message></messages></fix>",
            )
            .unwrap_err();

            assert!(matches!(error, Error::Invalid(_)));
            assert_eq!(
                error.to_string(),
                "invalid dictionary: message 'NewOrderSingle' references unknown component 'Parties'"
            );
        }

        #[test]
        fn malformed_xml() {
            let error = parse("<fix major='4'").unwrap_err();
            assert!(matches!(error, Error::Xml(_)));
        }

        #[test]
        fn unexpected_root() {
            let error = parse("<quickfix/>").unwrap_err();
            assert!(matches!(error, Error::UnexpectedRoot(ref root) if root == "quickfix"));
        }

        #[test]
        fn unknown_protocol() {
            let error = parse("<fix type='FIXML' major='4' minor='4'/>").unwrap_err();
            assert!(matches!(error, Error::UnknownProtocol(ref protocol) if protocol == "FIXML"));
        }

        #[test]
        fn missing_major() {
            let error = parse("<fix minor='4'/>").unwrap_err();
            assert!(matches!(
                error,
                Error::MissingAttribute { ref element, ref attribute }
                    if element == "fix" && attribute == "major"
            ));
        }

        #[test]
        fn non_numeric_major() {
            let error = parse("<fix major='four' minor='4'/>").unwrap_err();
            assert!(matches!(
                error,
                Error::InvalidNumber { ref element, ref attribute, ref value }
                    if element == "fix" && attribute == "major" && value == "four"
            ));
        }

        #[test]
        fn negative_major() {
            let error = parse("<fix major='-4' minor='4'/>").unwrap_err();
            assert!(matches!(error, Error::InvalidNumber { ref value, .. } if value == "-4"));
        }

        #[test]
        fn oversized_major() {
            let error = parse("<fix major='999' minor='4'/>").unwrap_err();
            assert!(matches!(error, Error::InvalidNumber { ref value, .. } if value == "999"));
        }

        #[test]
        fn errors_display_with_context() {
            let error = parse("<fix major='four' minor='4'/>").unwrap_err();
            assert_eq!(
                error.to_string(),
                "invalid number 'four' in attribute 'major' of <fix>"
            );
        }

        #[test]
        fn garbage_servicepack_is_an_error() {
            let error = parse("<fix major='4' minor='4' servicepack='abc'/>").unwrap_err();
            assert!(matches!(
                error,
                Error::InvalidNumber { ref attribute, ref value, .. }
                    if attribute == "servicepack" && value == "abc"
            ));
        }
    }
}
