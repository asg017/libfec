"""Typed itemization records: one class per schedule, with every field typed.

The classes are `fec-parser`'s own Rust structs
(`crates/fec-parser/src/itemizations/`), exposed as-is, like
:mod:`libfec.covers`.  Get one from ``Row.itemization``::

    import libfec as fec
    from libfec.itemizations import ScheduleA

    with fec.open("1926068.fec") as filing:
        for row in filing.rows("SA"):
            match row.itemization:
                case ScheduleA(memo=False) as sa:
                    print(sa.contributor.display_name(), sa.contribution_amount)

Every class is frozen (read-only) and has ``to_dict()``, ``__eq__`` and a short
``__repr__``; generated stubs (``itemizations.pyi``) give every field its type.
Names and addresses are :class:`libfec.covers.PersonName` and
:class:`libfec.covers.Address`.
"""

from typing import TypeAlias

from ._native import itemizations as _itemizations

Entity = _itemizations.Entity
CandidateRef = _itemizations.CandidateRef
ScheduleA = _itemizations.ScheduleA

#: What ``Row.itemization`` can be: one class per schedule `fec-parser` types.
Itemization: TypeAlias = ScheduleA

__all__ = [
    "Itemization",
    "Entity",
    "CandidateRef",
    "ScheduleA",
]
