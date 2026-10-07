//! The checked-in generated module stays fresh and drives typed access
//! end to end: dictionary XML to generated code to reads over a frame.

use bytes::Bytes;
use refix_codegen::rust::generate;
use refix_dictionary::quickfix;
use refix_message::{InvalidValue, Tag, Tokenizer};

// Not every generated item is read here, e.g. a leaf group's KNOWN_TAGS.
#[allow(dead_code)]
#[path = "data/toy_generated.rs"]
mod toy;

use toy::{ExecInst, Logon, NewOrderSingle, OrdType, PartyRole};

const TOY_XML: &str = include_str!("data/toy.xml");
const TOY_GENERATED: &str = include_str!("data/toy_generated.rs");

#[test]
fn the_checked_in_module_is_fresh() {
    let parsed = quickfix::parse(TOY_XML).unwrap();
    let generated = generate(&parsed.dictionary, "toy.xml").unwrap();
    assert_eq!(generated.code, TOY_GENERATED);
}

#[test]
fn the_toy_generates_without_warnings() {
    let parsed = quickfix::parse(TOY_XML).unwrap();
    let generated = generate(&parsed.dictionary, "toy.xml").unwrap();

    assert!(parsed.warnings.is_empty());
    assert!(generated.warnings.is_empty());
}

/// A complete frame around the `|`-delimited `body`, with correct
/// BodyLength and CheckSum.
fn frame(body: &str) -> Bytes {
    let body = body.replace('|', "\x01");
    let mut bytes = format!("8=FIX.4.4\x019={}\x01", body.len()).into_bytes();
    bytes.extend_from_slice(body.as_bytes());
    let sum = bytes.iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
    bytes.extend_from_slice(format!("10={sum:03}\x01").as_bytes());
    Bytes::from(bytes)
}

#[test]
fn typed_reads_over_a_tokenized_frame() {
    let raw = Tokenizer::default()
        .tokenize(frame("35=D|11=ORDER-1|38=200|44=101.5|40=1|"))
        .unwrap();

    let order = NewOrderSingle::from_raw(raw);

    assert_eq!(NewOrderSingle::MSG_TYPE, b"D");
    assert_eq!(order.raw().get(Tag(35)), Some(b"D".as_slice()));
    assert_eq!(order.cl_ord_id(), Ok(Some("ORDER-1")));
    assert_eq!(order.order_qty(), Ok(Some(200)));
    assert_eq!(order.price_raw(), Some(b"101.5".as_slice()));
    assert_eq!(order.ord_type(), Ok(Some(OrdType::Market)));
}

#[test]
fn an_absent_field_reads_as_none() {
    let raw = Tokenizer::default()
        .tokenize(frame("35=D|11=ORDER-1|"))
        .unwrap();

    let order = NewOrderSingle::from_raw(raw);

    assert_eq!(order.order_qty(), Ok(None));
    assert_eq!(order.price_raw(), None);
    assert_eq!(order.ord_type(), Ok(None));
}

#[test]
fn an_unrecognized_enum_value_is_representable() {
    let raw = Tokenizer::default().tokenize(frame("35=D|40=X|")).unwrap();

    let order = NewOrderSingle::from_raw(raw);

    assert_eq!(order.ord_type(), Ok(Some(OrdType::Unrecognized("X"))));
}

#[test]
fn a_malformed_value_reads_as_an_error() {
    let raw = Tokenizer::default()
        .tokenize(frame("35=D|38=12x3|"))
        .unwrap();

    let order = NewOrderSingle::from_raw(raw);

    assert_eq!(order.order_qty(), Err(InvalidValue { tag: Tag(38) }));
}

#[test]
fn typed_reads_over_groups() {
    let raw = Tokenizer::default()
        .tokenize(frame(
            "35=D|11=ORDER-1|453=2|448=AL|802=1|523=DESK-1|448=BOB|38=200|",
        ))
        .unwrap();

    let order = NewOrderSingle::from_raw(raw);
    let parties = order.parties().unwrap();

    let ids: Vec<Option<&str>> = parties
        .iter()
        .map(|party| party.party_id().unwrap())
        .collect();
    assert_eq!(ids, [Some("AL"), Some("BOB")]);

    let sub_ids = parties.get(0).unwrap().ptys_sub_grp().unwrap();
    assert_eq!(sub_ids.len(), 1);
    assert_eq!(sub_ids.get(0).unwrap().party_sub_id(), Ok(Some("DESK-1")));
    assert!(parties.get(1).unwrap().ptys_sub_grp().unwrap().is_empty());

    assert_eq!(order.order_qty(), Ok(Some(200)));
}

#[test]
fn an_absent_group_reads_as_empty() {
    let raw = Tokenizer::default()
        .tokenize(frame("35=D|11=ORDER-1|"))
        .unwrap();

    let order = NewOrderSingle::from_raw(raw);

    assert!(order.parties().unwrap().is_empty());
}

#[test]
fn a_malformed_group_reads_as_an_error() {
    let raw = Tokenizer::default()
        .tokenize(frame("35=D|453=2|448=AL|38=200|"))
        .unwrap();

    let order = NewOrderSingle::from_raw(raw);

    assert_eq!(order.parties().unwrap_err(), InvalidValue { tag: Tag(453) });
}

#[test]
fn a_header_field_after_a_group_ends_it() {
    let raw = Tokenizer::default()
        .tokenize(frame("35=A|384=1|372=D|49=SENDER|"))
        .unwrap();

    let logon = Logon::from_raw(raw);

    let msg_types = logon.msg_types().unwrap();
    assert_eq!(msg_types.get(0).unwrap().raw().get(Tag(49)), None);
}

#[test]
fn reads_a_group_declared_in_its_message() {
    let raw = Tokenizer::default()
        .tokenize(frame("35=A|384=2|372=D|372=8|"))
        .unwrap();

    let logon = Logon::from_raw(raw);

    let msg_types: Vec<Option<&str>> = logon
        .msg_types()
        .unwrap()
        .iter()
        .map(|msg_type| msg_type.ref_msg_type().unwrap())
        .collect();
    assert_eq!(msg_types, [Some("D"), Some("8")]);
}

#[test]
fn an_int_coded_enum_reads_by_value() {
    let raw = Tokenizer::default()
        .tokenize(frame(
            "35=D|453=3|448=AL|452=3|448=BO|452=03|448=CY|452=99|",
        ))
        .unwrap();

    let order = NewOrderSingle::from_raw(raw);

    let roles: Vec<Option<PartyRole>> = order
        .parties()
        .unwrap()
        .iter()
        .map(|party| party.party_role().unwrap())
        .collect();
    assert_eq!(
        roles,
        [
            Some(PartyRole::ClientId),
            Some(PartyRole::ClientId),
            Some(PartyRole::Unrecognized(99)),
        ]
    );
    assert_eq!(PartyRole::ClientId.value(), 3);
}

#[test]
fn a_non_integer_int_coded_enum_is_an_error() {
    let raw = Tokenizer::default()
        .tokenize(frame("35=D|453=1|448=AL|452=X|"))
        .unwrap();

    let order = NewOrderSingle::from_raw(raw);
    let parties = order.parties().unwrap();

    assert_eq!(
        parties.get(0).unwrap().party_role(),
        Err(InvalidValue { tag: Tag(452) })
    );
}

#[test]
fn a_multiple_value_field_reads_every_value() {
    let raw = Tokenizer::default()
        .tokenize(frame("35=D|18=1 6 Z|"))
        .unwrap();

    let order = NewOrderSingle::from_raw(raw);
    let exec_inst = order.exec_inst().unwrap().unwrap();

    let values: Vec<ExecInst<'_>> = exec_inst.iter().collect();
    assert_eq!(
        values,
        [
            ExecInst::NotHeld,
            ExecInst::ParticipateDontInitiate,
            ExecInst::Unrecognized("Z"),
        ]
    );
    assert!(exec_inst.contains(ExecInst::NotHeld));
}

#[test]
fn badly_spaced_multiple_values_are_an_error() {
    let raw = Tokenizer::default()
        .tokenize(frame("35=D|18=1  6|"))
        .unwrap();

    let order = NewOrderSingle::from_raw(raw);

    assert_eq!(
        order.exec_inst().unwrap_err(),
        InvalidValue { tag: Tag(18) }
    );
}
