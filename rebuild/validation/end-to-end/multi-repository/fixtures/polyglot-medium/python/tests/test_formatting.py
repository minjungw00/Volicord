from relay_formatter import format_message


def test_formats_message() -> None:
    assert format_message(" Ada ", "en") == {"message": "Hello, Ada"}
