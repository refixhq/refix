# refix-fix42

Typed FIX 4.2 messages for [ReFIX](https://refix.dev),
generated from QuickFIX's `FIX42.xml`.

```pycon
>>> from refix import Tokenizer
>>> from refix_fix42 import from_raw
>>> frame = (
...     "8=FIX.4.2|9=123|35=D|49=SENDER|56=TARGET|34=1|52=20261008-09:30:00|"
...     "11=ORDER-1|21=1|55=ACME|54=1|60=20261008-09:30:00|40=2|44=101.25|38=100|10=142|"
... )
>>> order = from_raw(Tokenizer().tokenize(frame.replace("|", "\x01").encode()))
>>> type(order).__name__
'NewOrderSingle'
>>> order.header.sender_comp_id
'SENDER'
>>> order.header.msg_seq_num
1
>>> order.cl_ord_id
'ORDER-1'
>>> order.handl_inst
<HandlInst.AUTOMATED_EXECUTION_NO_INTERVENTION: '1'>
>>> order.side
<Side.BUY: '1'>
>>> order.ord_type
<OrdType.LIMIT: '2'>
>>> order.price
Decimal('101.25')
>>> order.transact_time
datetime.datetime(2026, 10, 8, 9, 30, tzinfo=datetime.timezone.utc)
>>> order.transact_time_ns
1791451800000000000

```

`from_raw` reads each message as the class its MsgType names, and returns any
other unchanged.

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
