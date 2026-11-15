import inspect

import pytest
from refix import _core


def public_items() -> list[tuple[str, object]]:
    """Every public function, class and class member of the extension module."""
    items: list[tuple[str, object]] = []
    for name, item in vars(_core).items():
        if name.startswith("_") or not callable(item):
            continue
        items.append((name, item))
        if inspect.isclass(item):
            items.extend(
                (f"{name}.{member_name}", member)
                for member_name, member in vars(item).items()
                if not member_name.startswith("_")
            )
    return items


@pytest.mark.parametrize(
    "item", [pytest.param(item, id=name) for name, item in public_items()]
)
def test_is_documented(item: object):
    assert item.__doc__
