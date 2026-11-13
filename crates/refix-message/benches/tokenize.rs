use bytes::Bytes;
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use refix_message::{RawMessage, Tag, Tokenizer};
use std::hint::black_box;

fn read_message_raw(c: &mut Criterion) {
    let tokenizer = Tokenizer::default();
    let frame = frame(EXECUTION_REPORT);

    let message = tokenizer.tokenize(frame.clone()).unwrap();
    let expected: [&[u8]; 4] = [b"CLT0001", b"1000000", b"1.0925", b"20260731-10:00:00.120"];
    assert_eq!(read(&message), expected.map(Some));

    c.bench_function("read_message_raw", |b| {
        b.iter(|| {
            let message = tokenizer.tokenize(black_box(frame.clone())).unwrap();
            black_box(read(&message));
        })
    });
}

fn parse(c: &mut Criterion) {
    let tokenizer = Tokenizer::default();
    let mut group = c.benchmark_group("parse");

    for entries in [0, 10, 50, 200] {
        let frame = snapshot(entries);
        let fields = 10 + 3 * entries;
        assert_eq!(
            tokenizer.tokenize(frame.clone()).unwrap().entries().count(),
            fields
        );

        group.throughput(Throughput::Elements(fields as u64));
        group.bench_with_input(BenchmarkId::from_parameter(fields), &frame, |b, frame| {
            b.iter(|| tokenizer.tokenize(black_box(frame.clone())))
        });
    }

    group.finish();
}

const EXECUTION_REPORT: &str = "35=8|34=123|49=BROKER|56=CLIENT|52=20260731-10:00:00.123|\
      37=ORD0001|11=CLT0001|17=EXEC0001|150=F|39=2|1=ACC001|55=EURUSD|54=1|\
      38=1000000|40=2|44=1.0925|15=USD|59=0|32=1000000|31=1.0925|30=XOFF|\
      151=0|14=1000000|6=1.0925|75=20260731|64=20260804|60=20260731-10:00:00.120|";

const TAGS: [Tag; 4] = [Tag(11), Tag(38), Tag(44), Tag(60)];

fn frame(body: &str) -> Bytes {
    let body = body.replace("|", "\x01");
    let mut bytes = format!("8=FIX.4.4\x019={}\x01{body}", body.len()).into_bytes();
    let checksum = bytes.iter().fold(0u8, |sum, &b| sum.wrapping_add(b));
    bytes.extend_from_slice(format!("10={checksum:03}\x01").as_bytes());

    Bytes::from(bytes)
}

fn snapshot(entries: usize) -> Bytes {
    let mut body =
        format!("35=W|34=7|49=BROKER|56=CLIENT|52=20260731-10:00:00.123|55=EURUSD|268={entries}|");
    for i in 0..entries {
        body += &format!("269={}|270=1.{:04}|271={}|", i % 2, 925 + i, 1_000_000 + i);
    }

    frame(&body)
}

fn read(message: &RawMessage) -> [Option<&[u8]>; 4] {
    TAGS.map(|tag| message.get(tag))
}

criterion_group!(benches, read_message_raw, parse);
criterion_main!(benches);
