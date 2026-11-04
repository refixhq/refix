from io import BytesIO
from pathlib import Path

import pytest
import refix
from refix._core import _LogReader
from test_tokenizer import construct_valid_frame


class TestLogReader:
    def test_hands_over_prefix_message_and_diagnostics(self):
        frame = construct_valid_frame("FIX.4.4", "35=0|58=hello|")
        rewritten = b"8=FIX.4.4\x019=5\x0135=0\x0110=000\x01"
        reader = _LogReader()
        reader.feed(b"IN " + frame + b"\nOUT " + rewritten + b"\n")

        record = reader.next_record()
        records = reader.finish()

        assert record is not None
        prefix, message, diagnostics = record
        assert (prefix, message.bytes, diagnostics) == (b"IN ", frame, [])
        assert records[0][0] == b"OUT "
        assert records[0][2] == ["checksum_mismatch"]

    def test_reads_with_the_given_separator(self):
        reader = _LogReader(separator=ord("|"))
        reader.feed(b"8=FIX.4.4|9=5|35=0|10=163|")

        [(_, message, diagnostics)] = reader.finish()

        assert message.get(35) == b"0"
        assert diagnostics == []

    def test_cannot_feed_after_finish(self):
        reader = _LogReader()
        reader.finish()

        with pytest.raises(RuntimeError):
            reader.feed(b"")


def messages_of(
    outcomes: list[refix.RawMessage | refix.Garble],
) -> list[refix.RawMessage]:
    return [outcome for outcome in outcomes if isinstance(outcome, refix.RawMessage)]


class TestReadLog:
    def test_reads_messages_from_a_path(self, tmp_path: Path):
        first = construct_valid_frame("FIX.4.4", "35=0|58=first|")
        second = construct_valid_frame("FIX.4.4", "35=0|58=second|")
        path = tmp_path / "messages.log"
        path.write_bytes(first + second)

        outcomes = list(refix.read_log(path))

        assert [message.bytes for message in messages_of(outcomes)] == [first, second]

    def test_reads_from_a_file_object(self):
        frame = construct_valid_frame("FIX.4.4", "35=0|")

        outcomes = list(refix.read_log(BytesIO(frame)))

        assert len(outcomes) == 1
        assert messages_of(outcomes)[0].bytes == frame

    def test_timestamped_log_yields_messages_and_garbles(self):
        frames = [
            construct_valid_frame("FIX.4.4", "35=0|58=first|"),
            construct_valid_frame("FIX.4.4", "35=0|58=second|"),
        ]
        log = b"".join(
            b"20260803-10:00:00.%03d : " % i + frame + b"\n"
            for i, frame in enumerate(frames)
        )

        outcomes = list(refix.read_log(BytesIO(log)))

        assert [message.bytes for message in messages_of(outcomes)] == frames
        assert b"".join(outcome.bytes for outcome in outcomes) == log

    def test_truncated_tail_is_ignored(self):
        frame = construct_valid_frame("FIX.4.4", "35=0|58=hello|")
        log = frame + frame[: len(frame) // 2]

        outcomes = list(refix.read_log(BytesIO(log)))

        assert len(outcomes) == 1
        assert messages_of(outcomes)[0].bytes == frame

    def test_dialect_extras_flow_through(self):
        frame = construct_valid_frame("FIX.4.4", "35=0|5001=3|5002=a|b|")
        dialect = refix.Dialect(extra_length_tags=[5001])

        outcomes = list(refix.read_log(BytesIO(frame), dialect))

        assert messages_of(outcomes)[0].get(5002) == b"a\x01b"

    def test_log_larger_than_one_chunk(self):
        frame = construct_valid_frame("FIX.4.4", "35=0|58=hello|")
        count = 5000
        log = frame * count
        assert len(log) > 65536, "log must span several read chunks"

        outcomes = list(refix.read_log(BytesIO(log)))

        assert len(outcomes) == count
        assert all(message.bytes == frame for message in messages_of(outcomes))
        assert len(messages_of(outcomes)) == count


class TestDecodeLog:
    def test_decodes_each_message(self):
        frames = [
            construct_valid_frame("FIX.4.4", "35=0|58=first|"),
            construct_valid_frame("FIX.4.4", "35=0|58=second|"),
        ]
        codec = refix.Codec(refix.Dialect(), lambda raw: raw.get(58))

        decoded = list(refix.decode_log(BytesIO(b"".join(frames)), codec))

        assert decoded == [b"first", b"second"]

    def test_passes_garbles_through(self):
        frame = construct_valid_frame("FIX.4.4", "35=0|58=hello|")
        codec = refix.Codec(refix.Dialect(), lambda raw: raw.get(58))

        decoded = list(refix.decode_log(BytesIO(b"junk\n" + frame), codec))

        assert isinstance(decoded[0], refix.Garble)
        assert decoded[1:] == [b"hello"]

    def test_reads_in_the_codec_dialect(self):
        frame = construct_valid_frame("FIX.4.4", "35=0|5001=3|5002=a|b|")
        dialect = refix.Dialect(extra_length_tags=[5001])
        codec = refix.Codec(dialect, lambda raw: raw.get(5002))

        decoded = list(refix.decode_log(BytesIO(frame), codec))

        assert decoded == [b"a\x01b"]
