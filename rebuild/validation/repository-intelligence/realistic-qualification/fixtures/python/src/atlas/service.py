from atlas.core.model import Record, normalize


class Catalog:
    def create(self, value: str) -> Record:
        return Record(normalize(value))
