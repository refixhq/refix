from io import BytesIO
from pathlib import Path

import pytest
import refix
from refix._core import LogReader
from test_tokenizer import construct_valid_frame


class TestLogReader:
    def test_hands_over_prefix_message_and_diagnostics(self):
        frame = construct_valid_frame("FIX.4.4", "35=0|58=hello|")
        rewritten = b"8=FIX.4.4\x019=5\x0135=0\x0110=000\x01"
        reader = LogReader()
        reader.feed(b"IN " + frame + b"\nOUT " + rewritten + b"\n")

        record = reader.next_record()
        records = reader.finish()

        assert record is not None
        prefix, message, diagnostics = record
        assert (prefix, message.bytes, diagnostics) == (b"IN ", frame, [])
        assert records[0][0] == b"OUT "
        assert records[0][2] == ["checksum_mismatch"]

    def test_reads_with_the_given_separator(self):
        reader = LogReader(separator=ord("|"))
        reader.feed(b"8=FIX.4.4|9=5|35=0|10=163|")

        [(_, message, diagnostics)] = reader.finish()

        assert message.get(35) == b"0"
        assert diagnostics == []

    def test_cannot_feed_after_finish(self):
        reader = LogReader()
        reader.finish()

        with pytest.raises(RuntimeError):
            reader.feed(b"")


def messages_of(records: list[refix.LogRecord[refix.RawMessage]]) -> list[bytes]:
    return [record.message.bytes for record in records]


class TestReadLog:
    def test_reads_messages_from_a_path(self, tmp_path: Path):
        first = construct_valid_frame("FIX.4.4", "35=0|58=first|")
        second = construct_valid_frame("FIX.4.4", "35=0|58=second|")
        path = tmp_path / "messages.log"
        path.write_bytes(first + second)

        records = list(refix.read_log(path))

        assert messages_of(records) == [first, second]

    def test_reads_from_a_file_object(self):
        frame = construct_valid_frame("FIX.4.4", "35=0|")

        records = list(refix.read_log(BytesIO(frame)))

        assert messages_of(records) == [frame]

    def test_a_record_holds_the_prefix_and_diagnostics(self):
        frame = construct_valid_frame("FIX.4.4", "35=0|58=first|")
        cut = b"8=FIX.4.4\x019=20\x0135=0\x0158=hel"
        log = b"IN " + frame + b"\nOUT " + cut + b"\n"

        records = list(refix.read_log(BytesIO(log)))

        assert records == [
            refix.LogRecord(b"IN ", records[0].message, ()),
            refix.LogRecord(
                b"OUT ", records[1].message, (refix.LogDiagnostic.TRUNCATED,)
            ),
        ]
        assert messages_of(records) == [frame, cut]

    def test_reads_a_rewritten_separator(self):
        records = list(refix.read_log(BytesIO(b"8=FIX.4.4|9=5|35=0|10=163|\n")))

        assert records[0].message.get(35) == b"0"
        assert records[0].diagnostics == ()

    def test_reads_a_cat_v_copy(self):
        frame = construct_valid_frame("FIX.4.4", "35=0|58=hello|")
        shown = frame.replace(b"\x01", b"^A")

        records = list(refix.read_log(BytesIO(b"IN " + shown + b"\n")))

        assert records == [refix.LogRecord(b"IN ", records[0].message, ())]
        assert records[0].message.bytes == frame

    def test_a_given_separator_is_the_only_one_read(self):
        soh = construct_valid_frame("FIX.4.4", "35=0|")
        log = soh + b"\n8=FIX.4.4|9=5|35=0|10=163|\n"

        records = list(refix.read_log(BytesIO(log), separator=b"|"))

        assert [record.prefix for record in records] == [soh + b"\n"]

    def test_the_separator_must_be_one_byte(self):
        with pytest.raises(ValueError):
            next(refix.read_log(BytesIO(b""), separator=b"||"))

    def test_bytes_after_the_last_message_are_dropped(self):
        frame = construct_valid_frame("FIX.4.4", "35=0|58=hello|")

        records = list(refix.read_log(BytesIO(frame + b"\nshutting down\n")))

        assert messages_of(records) == [frame]

    def test_dialect_extras_flow_through(self):
        frame = construct_valid_frame("FIX.4.4", "35=0|5001=3|5002=a|b|")
        dialect = refix.Dialect(extra_length_tags=[5001])

        records = list(refix.read_log(BytesIO(frame), dialect))

        assert records[0].message.get(5002) == b"a\x01b"

    def test_log_larger_than_one_chunk(self):
        frame = construct_valid_frame("FIX.4.4", "35=0|58=hello|")
        count = 5000
        log = frame * count
        assert len(log) > 65536, "log must span several read chunks"

        records = list(refix.read_log(BytesIO(log)))

        assert len(records) == count
        assert all(record.message.bytes == frame for record in records)


class TestDecodeLog:
    def test_decodes_each_message(self):
        frames = [
            construct_valid_frame("FIX.4.4", "35=0|58=first|"),
            construct_valid_frame("FIX.4.4", "35=0|58=second|"),
        ]
        codec = refix.Codec(refix.Dialect(), lambda raw: raw.get(58))

        records = list(refix.decode_log(BytesIO(b"".join(frames)), codec))

        assert [record.message for record in records] == [b"first", b"second"]

    def test_keeps_the_prefix_and_diagnostics(self):
        codec = refix.Codec(refix.Dialect(), lambda raw: raw.get(35))

        records = list(
            refix.decode_log(BytesIO(b"junk\n8=FIX.4.4|9=5|35=0|10=000|"), codec)
        )

        assert records == [
            refix.LogRecord(b"junk\n", b"0", (refix.LogDiagnostic.CHECKSUM_MISMATCH,))
        ]

    def test_reads_in_the_codec_dialect(self):
        frame = construct_valid_frame("FIX.4.4", "35=0|5001=3|5002=a|b|")
        dialect = refix.Dialect(extra_length_tags=[5001])
        codec = refix.Codec(dialect, lambda raw: raw.get(5002))

        records = list(refix.decode_log(BytesIO(frame), codec, separator=b"\x01"))

        assert [record.message for record in records] == [b"a\x01b"]
