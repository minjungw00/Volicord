from greeter.core import Greeter


def test_greet() -> None:
    assert Greeter("hello").greet("Ada").startswith("hello")
