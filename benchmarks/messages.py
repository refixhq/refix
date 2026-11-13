def frame(body: str) -> bytes:
    payload = body.replace("|", "\x01").encode()
    head = b"8=FIX.4.4\x019=%d\x01" % len(payload)
    checksum = sum(head + payload) % 256
    return head + payload + b"10=%03d\x01" % checksum


def execution_report() -> bytes:
    return frame(_EXECUTION_REPORT)


_EXECUTION_REPORT = (
    "35=8|34=123|49=BROKER|56=CLIENT|52=20260731-10:00:00.123|"
    "37=ORD0001|11=CLT0001|17=EXEC0001|150=F|39=2|1=ACC001|55=EURUSD|54=1|"
    "38=1000000|40=2|44=1.0925|15=USD|59=0|32=1000000|31=1.0925|30=XOFF|"
    "151=0|14=1000000|6=1.0925|75=20260731|64=20260804|60=20260731-10:00:00.120|"
)
