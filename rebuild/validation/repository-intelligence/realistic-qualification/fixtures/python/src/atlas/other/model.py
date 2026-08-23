class Record:
    def __init__(self, number: int) -> None:
        self.number = number


def normalize(value: str) -> str:
    return value.casefold()
