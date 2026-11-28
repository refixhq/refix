//! A FIX engine written in pure Rust. Under active development, and APIs
//! change between 0.x releases.
//!
//! [`message`] frames, tokenises, reads and writes raw FIX messages. Typed
//! FIX 4.4 messages are in `refix-fix44`, and `refix-cli` generates them from
//! your own dictionary.
//!
//! ```
//! use refix::message::{LogReader, Tag};
//!
//! let mut reader = LogReader::default();
//! reader.feed(b"2026-10-10 09:30:00 IN 8=FIX.4.4|9=5|35=0|10=163|\n");
//! for record in reader.finish() {
//!     assert_eq!(record.message.get_str(Tag(35)), Ok(Some("0")));
//! }
//! ```

#[doc(inline)]
pub use refix_message as message;
