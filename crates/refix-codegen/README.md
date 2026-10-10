# refix-codegen

Generates typed FIX messages for [ReFIX](https://github.com/refixhq/refix), in Rust and Python, from a `Dictionary` read
by `refix-dictionary`.

> ReFIX is in early development, and APIs change between 0.x releases.

```rust
use refix_codegen::rust;
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
    let generated = rust::generate(&parsed.dictionary, "TestRequest.xml").unwrap();
    for warning in &generated.warnings {
        eprintln!("warning: {warning}");
    }
    print!("{}", generated.code);
}
```

The source name goes in the generated file's header. Rust output depends on `refix-message`, and `python::generate`
emits a module for `refix-engine` the same way. `refix-cli` runs both from the command line.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
