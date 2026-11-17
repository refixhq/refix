# refix-fix42

Typed FIX 4.2 messages for [ReFIX](https://refix.dev),
generated from QuickFIX's `FIX42.xml`.

```rust
use bytes::Bytes;
use refix::message::{Decimal, TimePrecision, Tokenizer, UtcTimestamp};
use refix_fix42::{HandlInst, Message, OrdType, Side};

let frame = "8=FIX.4.2|9=123|35=D|49=SENDER|56=TARGET|34=1|52=20261008-09:30:00|\
             11=ORDER-1|21=1|55=ACME|54=1|60=20261008-09:30:00|40=2|44=101.25|38=100|10=142|";
let raw = Tokenizer::default()
    .tokenize(Bytes::from(frame.replace('|', "\x01")))
    .unwrap();
let Message::NewOrderSingle(order) = Message::from_raw(raw) else {
    panic!("expected a NewOrderSingle");
};

assert_eq!(order.header().sender_comp_id(), Ok(Some("SENDER")));
assert_eq!(order.header().msg_seq_num(), Ok(Some(1)));
assert_eq!(order.cl_ord_id(), Ok(Some("ORDER-1")));
assert_eq!(
    order.handl_inst(),
    Ok(Some(HandlInst::AutomatedExecutionNoIntervention))
);
assert_eq!(order.symbol(), Ok(Some("ACME")));
assert_eq!(order.side(), Ok(Some(Side::Buy)));
assert_eq!(order.ord_type(), Ok(Some(OrdType::Limit)));
assert_eq!(order.price(), Ok(Some(Decimal::new(10125, -2))));
assert_eq!(
    order.transact_time(),
    Ok(Some(UtcTimestamp::new(1_791_451_800_000_000_000, TimePrecision::SECONDS)))
);
```

`Message::from_raw` reads each message as the type its MsgType names, and any
other as `Message::Unrecognized`.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  https://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or
  https://opensource.org/licenses/MIT)

at your option.

The code is generated from QuickFIX's `FIX42.xml`, under the QuickFIX Software
License ([LICENSE-QUICKFIX](LICENSE-QUICKFIX)). This product includes software
developed by quickfixengine.org (http://www.quickfixengine.org/).
