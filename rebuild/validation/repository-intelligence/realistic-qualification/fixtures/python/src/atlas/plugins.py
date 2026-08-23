from importlib import import_module
from typing import Callable, TypeVar

F = TypeVar("F", bound=Callable[..., object])


def traced(function: F) -> F:
    return function


@traced
def load_plugin(name: str) -> object:
    return import_module(name)
