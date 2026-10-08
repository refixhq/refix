# refix-fix44

Typed FIX 4.4 messages for [ReFIX](https://github.com/refixhq/refix),
generated from QuickFIX's `FIX44.xml`.

```pycon
>>> from refix import Tokenizer
>>> from refix_fix44 import NewOrderSingle
>>> frame = (
...     "8=FIX.4.4|9=118|35=D|49=SENDER|56=TARGET|34=1|52=20261008-09:30:00|"
...     "11=ORDER-1|55=ACME|54=1|60=20261008-09:30:00|40=2|44=101.25|38=100|10=194|"
... )
>>> order = NewOrderSingle(Tokenizer().tokenize(frame.replace("|", "\x01").encode()))
>>> order.header.sender_comp_id
'SENDER'
>>> order.header.msg_seq_num
1
>>> order.cl_ord_id
'ORDER-1'
>>> order.side
<Side.BUY: '1'>
>>> order.ord_type
<OrdType.LIMIT: '2'>
>>> order.price_raw
b'101.25'

```

Strings, integers, enums and repeating groups are typed, and every message
reaches its header and trailer. Prices, quantities and timestamps read as raw
bytes, as `price_raw` does.

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
