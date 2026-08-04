from io import BytesIO
from pathlib import Path

import refix
from test_tokenizer import construct_valid_frame


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

    def test_extra_length_tags_flow_through(self):
        frame = construct_valid_frame("FIX.4.4", "35=0|5001=3|5002=a|b|")

        outcomes = list(refix.read_log(BytesIO(frame), extra_length_tags=[5001]))

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
