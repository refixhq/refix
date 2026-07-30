/// `|`-delimited readable string into SOH-delimited bytes.
pub fn to_wire(s: &str) -> Vec<u8> {
    s.replace('|', "\x01").into_bytes()
}

/// A complete, correctly-checksummed frame with a correct BodyLength.
/// `body` is `|`-delimited and includes its trailing `|`, e.g. "35=A|34=1|".
pub fn construct_valid_frame(begin: &str, body: &str) -> Vec<u8> {
    let body = to_wire(body);
    let mut bytes = to_wire(&format!("8={begin}|9={}|", body.len()));
    bytes.extend_from_slice(&body);
    let cs = calculate_checksum(&bytes);
    bytes.extend_from_slice(&to_wire(&format!("10={cs}|")));
    bytes
}

/// FIX checksum: sum of all bytes mod 256, as a zero-padded 3-digit string.
pub fn calculate_checksum(bytes: &[u8]) -> String {
    let sum = bytes.iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
    format!("{sum:03}")
}
