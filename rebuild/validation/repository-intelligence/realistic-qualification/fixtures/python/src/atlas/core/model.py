from dataclasses import dataclass


@dataclass
class Record:
    value: str


def normalize(value: str) -> str:
    return value.strip()
