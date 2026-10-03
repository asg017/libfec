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
ScheduleB = _itemizations.ScheduleB
ScheduleD = _itemizations.ScheduleD
ScheduleFCommittee = _itemizations.ScheduleFCommittee
ScheduleF = _itemizations.ScheduleF
ScheduleH1 = _itemizations.ScheduleH1
ScheduleH2 = _itemizations.ScheduleH2
ScheduleH3 = _itemizations.ScheduleH3
ScheduleH4 = _itemizations.ScheduleH4
ScheduleH5 = _itemizations.ScheduleH5
ScheduleH6 = _itemizations.ScheduleH6
Form5Contribution = _itemizations.Form5Contribution
Form5Expenditure = _itemizations.Form5Expenditure
Form6Contribution = _itemizations.Form6Contribution
Form7Communication = _itemizations.Form7Communication
Form9ControllingPerson = _itemizations.Form9ControllingPerson
Form9Donation = _itemizations.Form9Donation
Form9Disbursement = _itemizations.Form9Disbursement
Form9Candidate = _itemizations.Form9Candidate
Form13Donation = _itemizations.Form13Donation
Form13Refund = _itemizations.Form13Refund
ScheduleL = _itemizations.ScheduleL
TextRecord = _itemizations.TextRecord
ScheduleA3L = _itemizations.ScheduleA3L
ScheduleE = _itemizations.ScheduleE
ScheduleC = _itemizations.ScheduleC
ScheduleCGuarantor = _itemizations.ScheduleCGuarantor
ScheduleC1 = _itemizations.ScheduleC1
ScheduleC2 = _itemizations.ScheduleC2

#: What ``Row.itemization`` can be: one class per record family `fec-parser`
#: types (``SA`` → ``ScheduleA``, ``H4`` → ``ScheduleH4``, ``TEXT`` → ``TextRecord``, …).
Itemization: TypeAlias = (
    ScheduleA
    | ScheduleB
    | ScheduleD
    | ScheduleF
    | ScheduleH1
    | ScheduleH2
    | ScheduleH3
    | ScheduleH4
    | ScheduleH5
    | ScheduleH6
    | Form5Contribution
    | Form5Expenditure
    | Form6Contribution
    | Form7Communication
    | Form9ControllingPerson
    | Form9Donation
    | Form9Disbursement
    | Form9Candidate
    | Form13Donation
    | Form13Refund
    | ScheduleL
    | TextRecord
    | ScheduleA3L
    | ScheduleE
    | ScheduleC
    | ScheduleC1
    | ScheduleC2
)

__all__ = [
    "Itemization",
    "Entity",
    "CandidateRef",
    "ScheduleA",
    "ScheduleB",
    "ScheduleD",
    "ScheduleFCommittee",
    "ScheduleF",
    "ScheduleH1",
    "ScheduleH2",
    "ScheduleH3",
    "ScheduleH4",
    "ScheduleH5",
    "ScheduleH6",
    "Form5Contribution",
    "Form5Expenditure",
    "Form6Contribution",
    "Form7Communication",
    "Form9ControllingPerson",
    "Form9Donation",
    "Form9Disbursement",
    "Form9Candidate",
    "Form13Donation",
    "Form13Refund",
    "ScheduleL",
    "TextRecord",
    "ScheduleA3L",
    "ScheduleE",
    "ScheduleC",
    "ScheduleCGuarantor",
    "ScheduleC1",
    "ScheduleC2",
]
