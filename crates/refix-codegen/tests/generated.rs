//! The checked-in generated module stays fresh and drives typed access
//! end to end: dictionary XML to generated code to reads over a frame.

use bytes::Bytes;
use refix_codegen::generate;
use refix_dictionary::quickfix;
use refix_message::{InvalidValue, Tokenizer};

#[path = "data/toy_generated.rs"]
mod toy;

use toy::NewOrderSingle;

const TOY_XML: &str = include_str!("data/toy.xml");
const TOY_GENERATED: &str = include_str!("data/toy_generated.rs");

#[test]
fn the_checked_in_module_is_fresh() {
    let parsed = quickfix::parse(TOY_XML).unwrap();
    let generated = generate(&parsed.dictionary, "toy.xml").unwrap();
    assert_eq!(generated, TOY_GENERATED);
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
        .tokenize(frame("35=D|11=ORDER-1|38=200|44=101.5|"))
        .unwrap();

    let order = NewOrderSingle::from_raw(raw);

    assert_eq!(NewOrderSingle::MSG_TYPE, b"D");
    assert_eq!(order.raw().get(35), Some(b"D".as_slice()));
    assert_eq!(order.cl_ord_id(), Ok(Some("ORDER-1")));
    assert_eq!(order.order_qty(), Ok(Some(200)));
    assert_eq!(order.price_raw(), Some(b"101.5".as_slice()));
}

#[test]
fn an_absent_field_reads_as_none() {
    let raw = Tokenizer::default()
        .tokenize(frame("35=D|11=ORDER-1|"))
        .unwrap();

    let order = NewOrderSingle::from_raw(raw);

    assert_eq!(order.order_qty(), Ok(None));
    assert_eq!(order.price_raw(), None);
}

#[test]
fn a_malformed_value_reads_as_an_error() {
    let raw = Tokenizer::default()
        .tokenize(frame("35=D|38=12x3|"))
        .unwrap();

    let order = NewOrderSingle::from_raw(raw);

    assert_eq!(order.order_qty(), Err(InvalidValue { tag: 38 }));
}
