use crate::{Dictionary, Protocol, Version};
use roxmltree::Node;
use std::num::ParseIntError;
use std::str::FromStr;

pub fn parse(xml: &str) -> Result<Parsed, Error> {
    let document = roxmltree::Document::parse(xml).map_err(Error::Xml)?;
    let root = document.root_element();
    if !root.has_tag_name("fix") {
        return Err(Error::UnexpectedRoot(root.tag_name().name().to_owned()));
    }

    let version = parse_version(root)?;

    let dictionary = Dictionary {
        version,
        messages: vec![],
        fields: vec![],
    };

    Ok(Parsed {
        dictionary,
        warnings: vec![],
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

#[derive(Debug)]
pub enum Warning {}

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
}
