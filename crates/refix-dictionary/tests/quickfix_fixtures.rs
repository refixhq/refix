//! The committed QuickFIX dictionary files are well-formed XML with the
//! expected top-level shape, so frontend work starts from known-good input.

const FIX44: &str = include_str!("data/quickfix/FIX44.xml");

#[test]
fn fix44_is_well_formed_xml() {
    let document = roxmltree::Document::parse(FIX44).unwrap();
    let root = document.root_element();

    assert_eq!(root.tag_name().name(), "fix");
    assert_eq!(root.attribute("major"), Some("4"));
    assert_eq!(root.attribute("minor"), Some("4"));

    let messages = root
        .children()
        .find(|node| node.has_tag_name("messages"))
        .unwrap();
    assert_eq!(
        messages
            .children()
            .filter(|node| node.has_tag_name("message"))
            .count(),
        93
    );
}
