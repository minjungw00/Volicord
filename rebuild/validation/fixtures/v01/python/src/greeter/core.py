import math


def format_name(name: str) -> str:
    return name.strip()


class Greeter:
    def __init__(self, prefix: str) -> None:
        self.prefix = prefix

    def greet(self, person: str) -> str:
        return f"{self.prefix}, {format_name(person)} {math.floor(1.8)}"
