/// `|`-delimited readable string into SOH-delimited bytes.
pub fn to_wire(s: &str) -> Vec<u8> {
    s.replace('|', "\x01").into_bytes()
}
