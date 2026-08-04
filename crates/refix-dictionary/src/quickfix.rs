use crate::{Dictionary, Protocol, Version};

pub fn parse(xml: &str) -> Result<Parsed, Error> {
    let dictionary = Dictionary {
        version: Version {
            protocol: Protocol::Fix,
            major: 0,
            minor: 0,
            service_pack: 0,
        },
        messages: vec![],
        fields: vec![],
    };

    Ok(Parsed {
        dictionary,
        warnings: vec![],
    })
}

pub struct Parsed {
    pub dictionary: Dictionary,
    pub warnings: Vec<Warning>,
}

pub enum Warning {}

pub enum Error {}
