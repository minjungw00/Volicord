from atlas.service import Catalog


def test_create_normalizes_value() -> None:
    assert Catalog().create(" Ada ").value == "Ada"
