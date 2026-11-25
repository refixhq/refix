#![no_main]

use libfuzzer_sys::fuzz_target;
use refix_message::{FieldWriter, Tag, Tokenizer, WriteError, frame};

const SOH: u8 = 0x01;
const DATA_LENGTH: Tag = Tag(95);
const DATA: Tag = Tag(96);
const FRAMING: [Tag; 4] = [Tag(8), Tag(9), Tag(35), Tag(10)];

/// The fields of one part and what the tokenizer should read for them.
#[derive(Default)]
struct Part {
    writer: FieldWriter,
    written: Vec<(Tag, Vec<u8>)>,
}

// The input is fields as `tag=value`, each ended by `|`. An empty field ends
// the header and starts the body. Tag 96 is written as a data field after its
// length.
fuzz_target!(|data: &[u8]| {
    let mut header = Part::default();
    let mut body = Part::default();
    let mut part = &mut header;
    for field in data.split(|&byte| byte == b'|') {
        if field.is_empty() {
            part = &mut body;
        } else if let Some((tag, value)) = tag_and_value(field) {
            write(part, tag, value);
        }
    }

    let written = [header.written, body.written].concat();
    let reserved = written.iter().find(|(tag, _)| FRAMING.contains(tag));
    let framed = frame(
        "FIX.4.4",
        "D",
        &header.writer.finish(),
        &body.writer.finish(),
    );
    let message = match (framed, reserved) {
        (Ok(message), None) => message,
        (Err(WriteError::ReservedTag { .. }), Some(_)) => return,
        (Err(error), _) => panic!("frame failed with {error}"),
        (Ok(_), Some((tag, _))) => panic!("frame took field {tag}"),
    };

    let read = Tokenizer::default()
        .tokenize(message.bytes().clone())
        .expect("a framed message must tokenize");
    assert!(
        read.entries().eq(message.entries()),
        "the tokenizer must read the fields the writer indexed"
    );
    let fields: Vec<_> = read.entries().collect();
    let between = &fields[3..fields.len() - 1];
    assert!(
        between
            .iter()
            .copied()
            .eq(written.iter().map(|(tag, value)| (*tag, &value[..]))),
        "the fields between MsgType and CheckSum must be the ones written"
    );
});

/// The tag and value of `field`, when its tag is one the target writes.
///
/// A length tag written as a plain field would have the tokenizer read the
/// next value by its extent. The standard ones are 90 to 43111.
fn tag_and_value(field: &[u8]) -> Option<(Tag, &[u8])> {
    let at = field.iter().position(|&byte| byte == b'=')?;
    let tag: u32 = std::str::from_utf8(&field[..at]).ok()?.parse().ok()?;
    let writable = tag < 90 || tag == DATA.0 || tag > 50_000;
    writable.then_some((Tag(tag), &field[at + 1..]))
}

fn write(part: &mut Part, tag: Tag, value: &[u8]) {
    if tag == DATA {
        match part.writer.write_data(DATA_LENGTH, DATA, value) {
            Ok(()) => {
                let length = value.len().to_string().into_bytes();
                part.written.push((DATA_LENGTH, length));
                part.written.push((DATA, value.to_vec()));
            }
            Err(error) => {
                assert_eq!(error, WriteError::EmptyValue { tag });
                assert!(value.is_empty(), "only an empty data value is rejected");
            }
        }
        return;
    }
    match part.writer.write(tag, value) {
        Ok(()) => {
            assert!(!value.is_empty() && !value.contains(&SOH));
            part.written.push((tag, value.to_vec()));
        }
        Err(error) if value.is_empty() => assert_eq!(error, WriteError::EmptyValue { tag }),
        Err(error) => {
            assert_eq!(error, WriteError::SeparatorInValue { tag });
            assert!(
                value.contains(&SOH),
                "a value without the separator was rejected"
            );
        }
    }
}
