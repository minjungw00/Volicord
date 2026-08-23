SCHEMA = "greeting.request.v1"
INBOX = "var/inbox/greeting.request.v1.json"


def format_request(name: str) -> str:
    return f"{SCHEMA}:{name.strip()}"
