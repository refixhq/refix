use criterion::{Criterion, criterion_group, criterion_main};
use refix_message::{
    Date, Decimal, FieldWriter, RawMessage, Tag, TimePrecision, UtcTimestamp, frame,
};
use std::hint::black_box;

fn write_message_raw(c: &mut Criterion) {
    assert_eq!(write_raw().to_string(), EXECUTION_REPORT);

    c.bench_function("write_message_raw", |b| b.iter(|| black_box(write_raw())));
}

fn write_message_typed(c: &mut Criterion) {
    assert_eq!(write_typed().to_string(), EXECUTION_REPORT);

    c.bench_function("write_message_typed", |b| {
        b.iter(|| black_box(write_typed()))
    });
}

fn frame_message(c: &mut Criterion) {
    let (header, body) = (typed_header(), typed_body());

    c.bench_function("frame", |b| {
        b.iter(|| frame("FIX.4.4", "8", black_box(&header), black_box(&body)))
    });
}

const EXECUTION_REPORT: &str = "8=FIX.4.4|9=268|35=8|34=123|49=BROKER|56=CLIENT|\
    52=20260731-10:00:00.123|37=ORD0001|11=CLT0001|17=EXEC0001|150=F|39=2|1=ACC001|55=EURUSD|\
    54=1|38=1000000|40=2|44=1.0925|15=USD|59=0|32=1000000|31=1.0925|30=XOFF|151=0|14=1000000|\
    6=1.0925|75=20260731|64=20260804|60=20260731-10:00:00.120|10=153";

const HEADER: [(Tag, &[u8]); 4] = [
    (Tag(34), b"123"),
    (Tag(49), b"BROKER"),
    (Tag(56), b"CLIENT"),
    (Tag(52), b"20260731-10:00:00.123"),
];

const BODY: [(Tag, &[u8]); 22] = [
    (Tag(37), b"ORD0001"),
    (Tag(11), b"CLT0001"),
    (Tag(17), b"EXEC0001"),
    (Tag(150), b"F"),
    (Tag(39), b"2"),
    (Tag(1), b"ACC001"),
    (Tag(55), b"EURUSD"),
    (Tag(54), b"1"),
    (Tag(38), b"1000000"),
    (Tag(40), b"2"),
    (Tag(44), b"1.0925"),
    (Tag(15), b"USD"),
    (Tag(59), b"0"),
    (Tag(32), b"1000000"),
    (Tag(31), b"1.0925"),
    (Tag(30), b"XOFF"),
    (Tag(151), b"0"),
    (Tag(14), b"1000000"),
    (Tag(6), b"1.0925"),
    (Tag(75), b"20260731"),
    (Tag(64), b"20260804"),
    (Tag(60), b"20260731-10:00:00.120"),
];

fn write_raw() -> RawMessage {
    let mut header = FieldWriter::new();
    for (tag, value) in HEADER {
        header.write(tag, value).unwrap();
    }
    let mut body = FieldWriter::new();
    for (tag, value) in BODY {
        body.write(tag, value).unwrap();
    }

    frame("FIX.4.4", "8", &header.finish(), &body.finish()).unwrap()
}

fn write_typed() -> RawMessage {
    frame("FIX.4.4", "8", &typed_header(), &typed_body()).unwrap()
}

fn typed_header() -> RawMessage {
    let sent = UtcTimestamp::new(1_785_492_000_123_000_000, TimePrecision::MILLIS);

    let mut writer = FieldWriter::new();
    writer.write_int(Tag(34), 123);
    writer.write_str(Tag(49), "BROKER").unwrap();
    writer.write_str(Tag(56), "CLIENT").unwrap();
    writer.write_utc_timestamp(Tag(52), sent);
    writer.finish()
}

fn typed_body() -> RawMessage {
    let price = Decimal::new(10925, -4);
    let traded = UtcTimestamp::new(1_785_492_000_120_000_000, TimePrecision::MILLIS);

    let mut writer = FieldWriter::new();
    writer.write_str(Tag(37), "ORD0001").unwrap();
    writer.write_str(Tag(11), "CLT0001").unwrap();
    writer.write_str(Tag(17), "EXEC0001").unwrap();
    writer.write_str(Tag(150), "F").unwrap();
    writer.write_str(Tag(39), "2").unwrap();
    writer.write_str(Tag(1), "ACC001").unwrap();
    writer.write_str(Tag(55), "EURUSD").unwrap();
    writer.write_str(Tag(54), "1").unwrap();
    writer.write_int(Tag(38), 1_000_000);
    writer.write_str(Tag(40), "2").unwrap();
    writer.write_decimal(Tag(44), price);
    writer.write_str(Tag(15), "USD").unwrap();
    writer.write_str(Tag(59), "0").unwrap();
    writer.write_int(Tag(32), 1_000_000);
    writer.write_decimal(Tag(31), price);
    writer.write_str(Tag(30), "XOFF").unwrap();
    writer.write_int(Tag(151), 0);
    writer.write_int(Tag(14), 1_000_000);
    writer.write_decimal(Tag(6), price);
    writer.write_date(Tag(75), Date::new(2026, 7, 31).unwrap());
    writer.write_date(Tag(64), Date::new(2026, 8, 4).unwrap());
    writer.write_utc_timestamp(Tag(60), traded);
    writer.finish()
}

criterion_group!(
    benches,
    write_message_raw,
    write_message_typed,
    frame_message
);
criterion_main!(benches);
