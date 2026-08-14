def normalize(value: str) -> str:
    return " ".join(value.strip().split())


def format_message(name: str, locale: str) -> dict[str, str]:
    greeting = "Hello" if locale == "en" else "안녕하세요"
    return {"message": f"{greeting}, {normalize(name)}"}
