import refix
from refix.errors import GarbledReason
from test_tokenizer import construct_valid_frame, to_wire


def drain(stream: refix.MessageStream) -> list[refix.RawMessage | refix.Garble]:
    """Collects outcomes until the stream reports no complete message."""
    outcomes: list[refix.RawMessage | refix.Garble] = []
    while (outcome := stream.next_message()) is not None:
        outcomes.append(outcome)
    return outcomes


def expect_message(outcome: object) -> refix.RawMessage:
    assert isinstance(outcome, refix.RawMessage)
    return outcome


def expect_garble(outcome: object) -> refix.Garble:
    assert isinstance(outcome, refix.Garble)
    return outcome


class TestDelivery:
    def test_empty_stream_has_no_message(self):
        assert refix.MessageStream().next_message() is None

    def test_single_message(self):
        frame = construct_valid_frame("FIX.4.4", "35=0|58=hi|")
        stream = refix.MessageStream()
        stream.feed(frame)

        outcomes = drain(stream)

        assert len(outcomes) == 1
        assert expect_message(outcomes[0]).bytes == frame

    def test_several_messages_in_one_feed(self):
        first = construct_valid_frame("FIX.4.4", "35=0|58=first|")
        second = construct_valid_frame("FIX.4.4", "35=0|58=second|")
        stream = refix.MessageStream()
        stream.feed(first)
        stream.feed(second)

        outcomes = drain(stream)

        assert len(outcomes) == 2
        assert expect_message(outcomes[0]).bytes == first
        assert expect_message(outcomes[1]).bytes == second

    def test_message_completes_across_feeds_at_every_split(self):
        frame = construct_valid_frame("FIX.4.4", "35=0|95=3|96=a|b|58=ok|")
        for split in range(1, len(frame)):
            stream = refix.MessageStream()
            stream.feed(frame[:split])
            assert stream.next_message() is None, f"prefix of {split} bytes"

            stream.feed(frame[split:])
            outcomes = drain(stream)
            assert len(outcomes) == 1, f"split at {split}"
            message = expect_message(outcomes[0])
            assert message.bytes == frame, f"split at {split}"
            assert message.get(96) == b"a\x01b"

    def test_dialect_extras_flow_through(self):
        frame = construct_valid_frame("FIX.4.4", "35=0|5001=3|5002=a|b|")
        stream = refix.MessageStream(extra_length_tags=[5001])
        stream.feed(frame)

        outcomes = drain(stream)

        assert expect_message(outcomes[0]).get(5002) == b"a\x01b"


class TestFaults:
    def test_garbage_between_messages(self):
        first = construct_valid_frame("FIX.4.4", "35=0|58=first|")
        second = construct_valid_frame("FIX.4.4", "35=0|58=second|")
        stream = refix.MessageStream()
        stream.feed(first)
        stream.feed(b"YY8=Z")
        stream.feed(second)

        outcomes = drain(stream)

        assert len(outcomes) == 3
        assert expect_message(outcomes[0]).bytes == first
        garble = expect_garble(outcomes[1])
        assert garble.reason is GarbledReason.MISSING_BEGIN_STRING
        assert garble.bytes == b"YY8=Z"
        assert expect_message(outcomes[2]).bytes == second

    def test_leading_garbage_is_skipped(self):
        frame = construct_valid_frame("FIX.4.4", "35=0|")
        stream = refix.MessageStream()
        stream.feed(b"junk")
        stream.feed(frame)

        outcomes = drain(stream)

        assert len(outcomes) == 2
        garble = expect_garble(outcomes[0])
        assert garble.reason is GarbledReason.MISSING_BEGIN_STRING
        assert garble.bytes == b"junk"
        assert expect_message(outcomes[1]).bytes == frame

    def test_garbled_frame_does_not_stall_the_stream(self):
        bad = bytearray(construct_valid_frame("FIX.4.4", "35=0|58=bad|"))
        bad[-2] = ord("1") if bad[-2] == ord("0") else ord("0")
        good = construct_valid_frame("FIX.4.4", "35=0|58=good|")
        stream = refix.MessageStream()
        stream.feed(bytes(bad))
        stream.feed(good)

        outcomes = drain(stream)

        assert expect_garble(outcomes[0]).reason is GarbledReason.CHECKSUM_MISMATCH
        assert expect_message(outcomes[-1]).bytes == good

    def test_oversized_frame_is_skipped_not_fatal(self):
        frame = construct_valid_frame("FIX.4.4", "35=0|")
        stream = refix.MessageStream()
        stream.feed(to_wire("8=FIX.4.4|9=1048577|"))
        stream.feed(frame)

        outcomes = drain(stream)

        assert expect_garble(outcomes[0]).reason is GarbledReason.FRAME_TOO_LARGE
        assert expect_message(outcomes[-1]).bytes == frame
