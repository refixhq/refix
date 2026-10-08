# refix-fix44

Typed FIX 4.4 messages for [ReFIX](https://github.com/refixhq/refix),
generated from QuickFIX's `FIX44.xml`.

```rust
use bytes::Bytes;
use refix_fix44::{NewOrderSingle, OrdType, Side};
use refix_message::Tokenizer;

let frame = "8=FIX.4.4|9=118|35=D|49=SENDER|56=TARGET|34=1|52=20261008-09:30:00|\
             11=ORDER-1|55=ACME|54=1|60=20261008-09:30:00|40=2|44=101.25|38=100|10=194|";
let raw = Tokenizer::default()
    .tokenize(Bytes::from(frame.replace('|', "\x01")))
    .unwrap();
let order = NewOrderSingle::from_raw(raw);

assert_eq!(order.header().sender_comp_id(), Ok(Some("SENDER")));
assert_eq!(order.header().msg_seq_num(), Ok(Some(1)));
assert_eq!(order.cl_ord_id(), Ok(Some("ORDER-1")));
assert_eq!(order.symbol(), Ok(Some("ACME")));
assert_eq!(order.side(), Ok(Some(Side::Buy)));
assert_eq!(order.ord_type(), Ok(Some(OrdType::Limit)));
assert_eq!(order.price_raw(), Some(b"101.25".as_slice()));
```

Strings, integers, enums and repeating groups are typed, and every message
reaches its header and trailer. Prices, quantities and timestamps read as raw
bytes, as `price_raw()` does.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  https://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or
  https://opensource.org/licenses/MIT)

at your option.

The code is generated from QuickFIX's `FIX44.xml`, under the QuickFIX Software
License ([LICENSE-QUICKFIX](LICENSE-QUICKFIX)). This product includes software
developed by quickfixengine.org (http://www.quickfixengine.org/).
