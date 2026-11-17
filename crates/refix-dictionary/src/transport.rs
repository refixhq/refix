//! Joining an application dictionary with its transport dictionary.
//!
//! From FIX 5.0 the header, the trailer and the session messages are in a
//! dictionary of their own, such as FIXT 1.1.

mod error;

use crate::Tag;
use crate::spec::{Field, Spec};
pub use error::Error;
use std::collections::{HashMap, HashSet};

impl Spec {
    /// Adds the header, trailer and session messages of `transport` to
    /// this application dictionary.
    ///
    /// A field or component in both keeps this dictionary's definition.
    /// The field also gains the values only the transport lists.
    pub fn with_transport(mut self, transport: Spec) -> Result<Spec, Error> {
        if !self.header.is_empty() {
            return Err(Error::ApplicationHeader);
        }
        if !self.trailer.is_empty() {
            return Err(Error::ApplicationTrailer);
        }
        if transport.header.is_empty() {
            return Err(Error::NoTransportHeader);
        }

        self.add_fields(transport.fields)?;
        let known: HashSet<String> = self
            .components
            .iter()
            .map(|component| component.name.clone())
            .collect();
        self.components.extend(
            transport
                .components
                .into_iter()
                .filter(|component| !known.contains(&component.name)),
        );
        let mut messages = transport.messages;
        messages.append(&mut self.messages);
        self.messages = messages;
        self.header = transport.header;
        self.trailer = transport.trailer;
        Ok(self)
    }

    fn add_fields(&mut self, transport: Vec<Field>) -> Result<(), Error> {
        let by_tag: HashMap<Tag, usize> = self
            .fields
            .iter()
            .enumerate()
            .map(|(index, field)| (field.tag, index))
            .collect();
        let by_name: HashMap<String, usize> = self
            .fields
            .iter()
            .enumerate()
            .map(|(index, field)| (field.name.clone(), index))
            .collect();

        for field in transport {
            let index = by_tag
                .get(&field.tag)
                .or_else(|| by_name.get(&field.name))
                .copied();
            let Some(index) = index else {
                self.fields.push(field);
                continue;
            };
            let ours = &mut self.fields[index];
            if ours.tag != field.tag || ours.name != field.name {
                return Err(Error::FieldMismatch {
                    application: (ours.name.clone(), ours.tag),
                    transport: (field.name, field.tag),
                });
            }
            ours.length_tag = ours.length_tag.or(field.length_tag);
            for value in field.values {
                let code = ours.data_type.canonical_code(&value.value);
                let is_listed = ours
                    .values
                    .iter()
                    .any(|listed| ours.data_type.canonical_code(&listed.value) == code);
                if !is_listed {
                    ours.values.push(value);
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Error;
    use crate::spec::{
        Category, Component, ComponentRef, DataType, EnumValue, Field, FieldRef, Member, Message,
        Protocol, Spec, Version,
    };
    use crate::{Tag, dictionary};

    fn field(name: &str, tag: u32) -> Field {
        Field {
            name: name.to_owned(),
            tag: Tag(tag),
            data_type: DataType::String,
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

    fn component_ref(name: &str) -> Member {
        Member::Component(ComponentRef {
            name: name.to_owned(),
            is_required: false,
        })
    }

    fn message(name: &str, msg_type: &str, category: Category, members: Vec<Member>) -> Message {
        Message {
            name: name.to_owned(),
            msg_type: msg_type.to_owned(),
            members,
            category,
        }
    }

    /// An application dictionary with one message and no envelope.
    fn application() -> Spec {
        Spec {
            version: Version {
                protocol: Protocol::Fix,
                major: 5,
                minor: 0,
                service_pack: 2,
            },
            header: vec![],
            messages: vec![message(
                "NewOrderSingle",
                "D",
                Category::App,
                vec![field_ref(11)],
            )],
            trailer: vec![],
            fields: vec![field("ClOrdID", 11)],
            components: vec![],
        }
    }

    /// A transport dictionary with a header, a trailer and one message.
    fn transport() -> Spec {
        Spec {
            version: Version {
                protocol: Protocol::Fixt,
                major: 1,
                minor: 1,
                service_pack: 0,
            },
            header: vec![field_ref(8), field_ref(49)],
            messages: vec![message(
                "Heartbeat",
                "0",
                Category::Admin,
                vec![field_ref(112)],
            )],
            trailer: vec![field_ref(10)],
            fields: vec![
                field("BeginString", 8),
                field("SenderCompID", 49),
                field("TestReqID", 112),
                field("CheckSum", 10),
            ],
            components: vec![],
        }
    }

    fn names(fields: &[Field]) -> Vec<&str> {
        fields.iter().map(|field| field.name.as_str()).collect()
    }

    #[test]
    fn takes_the_envelope_and_session_messages_from_the_transport() {
        let merged = application().with_transport(transport()).unwrap();

        assert_eq!(merged.version, application().version);
        assert_eq!(merged.header, transport().header);
        assert_eq!(merged.trailer, transport().trailer);
        let messages: Vec<&str> = merged
            .messages
            .iter()
            .map(|message| message.name.as_str())
            .collect();
        assert_eq!(messages, ["Heartbeat", "NewOrderSingle"]);
        assert_eq!(
            names(&merged.fields),
            [
                "ClOrdID",
                "BeginString",
                "SenderCompID",
                "TestReqID",
                "CheckSum"
            ]
        );
    }

    #[test]
    fn the_merged_dictionary_resolves() {
        let dictionary = application()
            .with_transport(transport())
            .unwrap()
            .resolve()
            .unwrap()
            .dictionary;

        assert_eq!(dictionary.header().count(), 2);
        assert_eq!(dictionary.trailer().count(), 1);
        let order = dictionary
            .messages()
            .find(|message| message.name() == "NewOrderSingle")
            .unwrap();
        assert!(matches!(
            order.members().next(),
            Some(dictionary::Member::Field { field, .. }) if field.tag == Tag(11)
        ));
    }

    #[test]
    fn a_field_in_both_keeps_the_application_definition() {
        let mut application = application();
        application.fields.push(Field {
            data_type: DataType::Other("XMLDATA".to_owned()),
            ..field("XmlData", 213)
        });
        let mut transport = transport();
        transport.fields.push(Field {
            data_type: DataType::Data,
            ..field("XmlData", 213)
        });

        let merged = application.with_transport(transport).unwrap();

        let xml_data: Vec<&Field> = merged
            .fields
            .iter()
            .filter(|field| field.tag == Tag(213))
            .collect();
        assert_eq!(xml_data.len(), 1);
        assert_eq!(xml_data[0].data_type, DataType::Other("XMLDATA".to_owned()));
    }

    #[test]
    fn a_field_in_both_gains_the_values_only_the_transport_lists() {
        let mut application = application();
        application.fields.push(with_values(
            Field {
                data_type: DataType::Int,
                ..field("SessionRejectReason", 373)
            },
            &[("9", "COMP_ID_PROBLEM")],
        ));
        let mut transport = transport();
        transport.fields.push(with_values(
            Field {
                data_type: DataType::Int,
                ..field("SessionRejectReason", 373)
            },
            &[
                ("09", "COMPID_PROBLEM"),
                ("18", "INVALID_APPLICATION_VERSION"),
            ],
        ));

        let merged = application.with_transport(transport).unwrap();

        let reason = merged
            .fields
            .iter()
            .find(|field| field.tag == Tag(373))
            .unwrap();
        let values: Vec<(&str, &str)> = reason
            .values
            .iter()
            .map(|value| (value.value.as_str(), value.description.as_str()))
            .collect();
        assert_eq!(
            values,
            [
                ("9", "COMP_ID_PROBLEM"),
                ("18", "INVALID_APPLICATION_VERSION")
            ]
        );
    }

    #[test]
    fn a_data_field_takes_the_length_tag_the_transport_found() {
        let mut application = application();
        application.fields.push(Field {
            data_type: DataType::Data,
            ..field("Signature", 89)
        });
        let mut transport = transport();
        transport.fields.push(Field {
            data_type: DataType::Data,
            length_tag: Some(Tag(93)),
            ..field("Signature", 89)
        });

        let merged = application.with_transport(transport).unwrap();

        let signature = merged
            .fields
            .iter()
            .find(|field| field.tag == Tag(89))
            .unwrap();
        assert_eq!(signature.length_tag, Some(Tag(93)));
    }

    #[test]
    fn a_component_in_both_keeps_the_application_definition() {
        let mut application = application();
        application.fields.push(field("RefMsgType", 372));
        application.components.push(Component {
            name: "MsgTypeGrp".to_owned(),
            members: vec![field_ref(372)],
        });
        let mut transport = transport();
        transport.components = vec![
            Component {
                name: "MsgTypeGrp".to_owned(),
                members: vec![],
            },
            Component {
                name: "HopGrp".to_owned(),
                members: vec![field_ref(49)],
            },
        ];
        transport.messages[0]
            .members
            .push(component_ref("MsgTypeGrp"));

        let merged = application.with_transport(transport).unwrap();

        let components: Vec<(&str, usize)> = merged
            .components
            .iter()
            .map(|component| (component.name.as_str(), component.members.len()))
            .collect();
        assert_eq!(components, [("MsgTypeGrp", 1), ("HopGrp", 1)]);
    }

    #[test]
    fn an_application_dictionary_with_a_header_is_an_error() {
        let mut application = application();
        application.header = vec![field_ref(11)];

        assert_eq!(
            application.with_transport(transport()),
            Err(Error::ApplicationHeader)
        );
    }

    #[test]
    fn an_application_dictionary_with_a_trailer_is_an_error() {
        let mut application = application();
        application.trailer = vec![field_ref(11)];

        assert_eq!(
            application.with_transport(transport()),
            Err(Error::ApplicationTrailer)
        );
    }

    #[test]
    fn a_transport_without_a_header_is_an_error() {
        let mut transport = transport();
        transport.header = vec![];

        assert_eq!(
            application().with_transport(transport),
            Err(Error::NoTransportHeader)
        );
    }

    #[test]
    fn one_tag_under_two_names_is_an_error() {
        let mut application = application();
        application.fields.push(field("Sender", 49));

        let error = application.with_transport(transport()).unwrap_err();

        assert_eq!(
            error,
            Error::FieldMismatch {
                application: ("Sender".to_owned(), Tag(49)),
                transport: ("SenderCompID".to_owned(), Tag(49)),
            }
        );
        assert_eq!(
            error.to_string(),
            "field 'Sender' (49) in the application dictionary is 'SenderCompID' (49) in the \
             transport"
        );
    }

    #[test]
    fn one_name_under_two_tags_is_an_error() {
        let mut application = application();
        application.fields.push(field("SenderCompID", 5049));

        assert_eq!(
            application.with_transport(transport()),
            Err(Error::FieldMismatch {
                application: ("SenderCompID".to_owned(), Tag(5049)),
                transport: ("SenderCompID".to_owned(), Tag(49)),
            })
        );
    }
}
