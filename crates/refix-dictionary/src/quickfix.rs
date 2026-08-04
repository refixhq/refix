use crate::{Category, DataType, Dictionary, Field, FieldRef, Message, Protocol, Version};
use roxmltree::Node;
use std::collections::HashMap;
use std::num::ParseIntError;
use std::str::FromStr;

pub fn parse(xml: &str) -> Result<Parsed, Error> {
    let document = roxmltree::Document::parse(xml).map_err(Error::Xml)?;
    let root = document.root_element();
    if !root.has_tag_name("fix") {
        return Err(Error::UnexpectedRoot(root.tag_name().name().to_owned()));
    }

    let mut warnings = Vec::new();
    let version = parse_version(root)?;
    let fields = parse_fields(root, &mut warnings)?;
    let messages = parse_messages(root, &fields, &mut warnings)?;

    let dictionary = Dictionary {
        version,
        messages,
        fields,
    };

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

    section
        .children()
        .filter(|node| node.has_tag_name("field"))
        .map(|node| parse_field(node, warnings))
        .collect()
}

fn parse_field(node: Node, warnings: &mut Vec<Warning>) -> Result<Field, Error> {
    let name = string_attribute(node, "name")?;
    if node.children().any(|child| child.has_tag_name("value")) {
        warnings.push(Warning::UnsupportedEnumValues {
            field: name.clone(),
        });
    }

    Ok(Field {
        name,
        tag: int_attribute(node, "number")?,
        data_type: parse_data_type(string_attribute(node, "type")?),
    })
}

fn parse_data_type(name: String) -> DataType {
    match name.as_str() {
        "STRING" => DataType::String,
        "INT" => DataType::Int,
        _ => DataType::Other(name),
    }
}

fn parse_messages(
    root: Node,
    fields: &[Field],
    warnings: &mut Vec<Warning>,
) -> Result<Vec<Message>, Error> {
    let Some(section) = root.children().find(|node| node.has_tag_name("messages")) else {
        return Ok(Vec::new());
    };

    let tags_by_name: HashMap<&str, u32> = fields
        .iter()
        .map(|field| (field.name.as_str(), field.tag))
        .collect();

    section
        .children()
        .filter(|node| node.has_tag_name("message"))
        .map(|node| parse_message(node, &tags_by_name, warnings))
        .collect()
}

fn parse_message(
    node: Node,
    tags_by_name: &HashMap<&str, u32>,
    warnings: &mut Vec<Warning>,
) -> Result<Message, Error> {
    let name = string_attribute(node, "name")?;
    let mut fields = Vec::new();
    let category = parse_category(node)?;

    for child in node.children().filter(Node::is_element) {
        match child.tag_name().name() {
            "field" => fields.push(parse_field_ref(child, &name, tags_by_name)?),
            "component" => warnings.push(Warning::UnsupportedComponent {
                message: name.clone(),
                component: string_attribute(child, "name")?,
            }),
            "group" => warnings.push(Warning::UnsupportedGroup {
                message: name.clone(),
                group: string_attribute(child, "name")?,
            }),
            other => warnings.push(Warning::UnsupportedElement {
                message: name.clone(),
                element: other.to_owned(),
            }),
        }
    }

    Ok(Message {
        name,
        msg_type: string_attribute(node, "msgtype")?,
        fields,
        category,
    })
}

fn parse_field_ref(
    node: Node,
    message: &str,
    tags_by_name: &HashMap<&str, u32>,
) -> Result<FieldRef, Error> {
    let name = string_attribute(node, "name")?;
    let Some(&tag) = tags_by_name.get(name.as_str()) else {
        return Err(Error::UnknownField {
            message: message.to_owned(),
            field: name,
        });
    };

    Ok(FieldRef {
        tag,
        is_required: required_attribute(node)?,
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

#[derive(Debug)]
pub struct Parsed {
    pub dictionary: Dictionary,
    pub warnings: Vec<Warning>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum Warning {
    UnsupportedEnumValues { field: String },
    UnsupportedComponent { message: String, component: String },
    UnsupportedGroup { message: String, group: String },
    UnsupportedElement { message: String, element: String },
    UnsupportedSection { section: String },
}

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
        message: String,
        field: String,
    },
    InvalidAttribute {
        element: String,
        attribute: String,
        value: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version_of(xml: &str) -> Version {
        parse(xml).unwrap().dictionary.version
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
            let fields = parse(DICTIONARY).unwrap().dictionary.fields;

            assert_eq!(
                fields,
                vec![
                    Field {
                        name: "ClOrdID".to_owned(),
                        tag: 11,
                        data_type: DataType::String,
                    },
                    Field {
                        name: "PriceType".to_owned(),
                        tag: 423,
                        data_type: DataType::Int,
                    },
                    Field {
                        name: "OrderQty".to_owned(),
                        tag: 38,
                        data_type: DataType::Other("QTY".to_owned()),
                    },
                    Field {
                        name: "OrdType".to_owned(),
                        tag: 40,
                        data_type: DataType::Other("CHAR".to_owned()),
                    },
                ]
            );
        }

        #[test]
        fn enum_values_surface_as_a_warning() {
            let parsed = parse(DICTIONARY).unwrap();

            assert_eq!(
                parsed.warnings,
                vec![Warning::UnsupportedEnumValues {
                    field: "OrdType".to_owned(),
                }]
            );
        }

        #[test]
        fn missing_fields_section_yields_no_fields() {
            let parsed = parse("<fix major='4' minor='4'/>").unwrap();
            assert!(parsed.dictionary.fields.is_empty());
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

    mod errors {
        use super::*;

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
