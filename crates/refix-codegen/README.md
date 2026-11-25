# refix-codegen

Generates typed FIX messages for [ReFIX](https://refix.dev), in Rust and Python, from a `Dictionary` read
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
    let package = rust::generate(&parsed.dictionary, "TestRequest.xml").unwrap();
    for warning in &package.warnings {
        eprintln!("warning: {warning}");
    }
    for module in &package.modules {
        println!("{}", module.path.display());
    }
}
```

The source name goes in the first line of every generated file. `Package::write` puts the files in a directory.
Rust output depends on `refix-message`. `python::generate` returns a package for the `refix` package on PyPI the same
way. `refix-cli` runs both from the command line.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
