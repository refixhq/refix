# refix-dictionary

The FIX dictionary model for [ReFIX](https://github.com/refixhq/refix), and a parser for QuickFIX XML dictionaries.
A `Dictionary` is resolved, so every reference in it is valid.

> ReFIX is in early development, and APIs change between 0.x releases.

```rust
use refix_dictionary::dictionary::Member;
use refix_dictionary::quickfix;

fn main() {
    let xml = "
<fix major='4' minor='4'>
  <messages>
    <message name='TestRequest' msgtype='1' msgcat='admin'>
      <field name='TestReqID' required='Y'/>
    </message>
  </messages>
  <fields>
    <field number='112' name='TestReqID' type='STRING'/>
  </fields>
</fix>";
    let parsed = quickfix::parse(xml).unwrap();
    for warning in &parsed.warnings {
        println!("warning: {warning}");
    }
    for message in parsed.dictionary.messages() {
        println!("{} (35={})", message.name(), message.msg_type());
        for member in message.members() {
            if let Member::Field { field, .. } = member {
                println!("  {}", field.name);
            }
        }
    }
}
```

Components are expanded in place, so a message lists every field and group it can hold. `refix-codegen` generates typed
messages from a `Dictionary`, and `refix-cli` runs it from the command line.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
