use bytes::Bytes;
use pyo3::{PyErr, PyResult, import_exception, pyclass, pymethods};
use refix_message::framing::GarbledReason;
use refix_message::{TokenizeError, Tokenizer as CoreTokenizer};

import_exception!(refix.errors, GarbledError);
import_exception!(refix.errors, IncompleteError);
import_exception!(refix.errors, TrailingBytesError);
import_exception!(refix.errors, TooLargeToIndexError);

#[pyclass(frozen, module = "refix._core")]
pub(crate) struct Tokenizer(CoreTokenizer);

#[pymethods]
impl Tokenizer {
    #[new]
    #[pyo3(signature = (*, extra_length_tags = Vec::new()))]
    fn new(extra_length_tags: Vec<u32>) -> Self {
        Self(CoreTokenizer::with_extra_length_tags(extra_length_tags))
    }

    fn tokenize(&self, data: &[u8]) -> PyResult<crate::message::RawMessage> {
        self.0
            .tokenize(Bytes::copy_from_slice(data))
            .map(crate::message::RawMessage::new)
            .map_err(to_py_err)
    }
}

fn to_py_err(error: TokenizeError) -> PyErr {
    match error {
        TokenizeError::Garbled(reason) => GarbledError::new_err((reason_str(reason),)),
        TokenizeError::Incomplete => IncompleteError::new_err(()),
        TokenizeError::TrailingBytes { frame_len } => TrailingBytesError::new_err((frame_len,)),
        TokenizeError::TooLargeToIndex => TooLargeToIndexError::new_err(()),
    }
}

fn reason_str(reason: GarbledReason) -> &'static str {
    match reason {
        GarbledReason::MissingBeginString => "missing_begin_string",
        GarbledReason::MalformedBeginString => "malformed_begin_string",
        GarbledReason::MissingBodyLength => "missing_body_length",
        GarbledReason::MalformedBodyLength => "malformed_body_length",
        GarbledReason::BodyLengthMismatch => "body_length_mismatch",
        GarbledReason::MissingMsgType => "missing_msg_type",
        GarbledReason::MalformedChecksum => "malformed_checksum",
        GarbledReason::ChecksumMismatch => "checksum_mismatch",
        GarbledReason::FrameTooLarge => "frame_too_large",
    }
}
