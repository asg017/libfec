"""Typed cover records: one class per FEC form, with every field typed.

The classes are `fec-parser`'s own Rust structs (`crates/fec-parser/src/covers/`),
exposed as-is: the same names, the same fields, the same field docs, and the
same shape as ``libfec info -f json``'s ``cover_data``.  Get one from
``FilingReader.cover_data`` / ``Filing.cover_data``::

    import libfec_parser as fec
    from libfec_parser.covers import Form3X

    with fec.open("1926068.fec") as filing:
        match filing.cover_data:
            case Form3X() as f3x:
                print(f3x.summary.line6c_total_receipts.column_a)
            case None:
                ...  # a form with no typed struct

Every class is frozen (read-only) and has ``to_dict()``, ``__eq__`` and a short
``__repr__``; generated stubs (``covers.pyi``) give every field its type.
"""

from typing import TypeAlias

from ._native import covers as _covers

PersonName = _covers.PersonName
Address = _covers.Address
DetailedSummaryRow = _covers.DetailedSummaryRow
Form1 = _covers.Form1
Form1PacFlags = _covers.Form1PacFlags
Form1Candidate = _covers.Form1Candidate
Form1Affiliated = _covers.Form1Affiliated
Form1Contact = _covers.Form1Contact
Form1Bank = _covers.Form1Bank
Form1M = _covers.Form1M
Form1MAffiliation = _covers.Form1MAffiliation
Form1MQualification = _covers.Form1MQualification
Form1MCandidate = _covers.Form1MCandidate
Form2 = _covers.Form2
Form2Committee = _covers.Form2Committee
Form2PersonalFundsDeclaration = _covers.Form2PersonalFundsDeclaration
Form3 = _covers.Form3
Form3Summary = _covers.Form3Summary
Form3DetailedSummary = _covers.Form3DetailedSummary
Form3DetailedSummaryReceipts = _covers.Form3DetailedSummaryReceipts
Form3DetailedSummaryDisbursements = _covers.Form3DetailedSummaryDisbursements
Form3CashSummary = _covers.Form3CashSummary
Form3L = _covers.Form3L
Form3P = _covers.Form3P
Form3PSummary = _covers.Form3PSummary
Form3PDetailedSummary = _covers.Form3PDetailedSummary
Form3PDetailedSummaryReceipts = _covers.Form3PDetailedSummaryReceipts
Form3PDetailedSummaryDisbursements = _covers.Form3PDetailedSummaryDisbursements
Form3PStateAllocations = _covers.Form3PStateAllocations
Form3PStateAllocation = _covers.Form3PStateAllocation
Form3X = _covers.Form3X
Form3XSummary = _covers.Form3XSummary
Form3XDetailedSummary = _covers.Form3XDetailedSummary
Form3XReceipts = _covers.Form3XReceipts
Form3XDisbursements = _covers.Form3XDisbursements
Form3XNetContributionsAndOperatingExpenditures = _covers.Form3XNetContributionsAndOperatingExpenditures
Form4 = _covers.Form4
Form4Summary = _covers.Form4Summary
Form4DetailedSummary = _covers.Form4DetailedSummary
Form4Receipts = _covers.Form4Receipts
Form4Disbursements = _covers.Form4Disbursements
Form4ItemizedLine = _covers.Form4ItemizedLine
Form4LoanLine = _covers.Form4LoanLine
Form5 = _covers.Form5
Form6 = _covers.Form6
Form6Candidate = _covers.Form6Candidate
Form7 = _covers.Form7
Form9 = _covers.Form9
Form9Custodian = _covers.Form9Custodian
Form13 = _covers.Form13
Form24 = _covers.Form24
Form99 = _covers.Form99

#: What ``cover_data`` can be: one class per form `fec-parser` types.  A
#: ``match`` or ``isinstance`` on it narrows to that form's class.
CoverData: TypeAlias = (
    Form1 | Form1M | Form2 | Form3 | Form3L | Form3P | Form3X | Form4 | Form5
    | Form6 | Form7 | Form9 | Form13 | Form24 | Form99
)

__all__ = [
    "CoverData",
    "PersonName",
    "Address",
    "DetailedSummaryRow",
    "Form1",
    "Form1PacFlags",
    "Form1Candidate",
    "Form1Affiliated",
    "Form1Contact",
    "Form1Bank",
    "Form1M",
    "Form1MAffiliation",
    "Form1MQualification",
    "Form1MCandidate",
    "Form2",
    "Form2Committee",
    "Form2PersonalFundsDeclaration",
    "Form3",
    "Form3Summary",
    "Form3DetailedSummary",
    "Form3DetailedSummaryReceipts",
    "Form3DetailedSummaryDisbursements",
    "Form3CashSummary",
    "Form3L",
    "Form3P",
    "Form3PSummary",
    "Form3PDetailedSummary",
    "Form3PDetailedSummaryReceipts",
    "Form3PDetailedSummaryDisbursements",
    "Form3PStateAllocations",
    "Form3PStateAllocation",
    "Form3X",
    "Form3XSummary",
    "Form3XDetailedSummary",
    "Form3XReceipts",
    "Form3XDisbursements",
    "Form3XNetContributionsAndOperatingExpenditures",
    "Form4",
    "Form4Summary",
    "Form4DetailedSummary",
    "Form4Receipts",
    "Form4Disbursements",
    "Form4ItemizedLine",
    "Form4LoanLine",
    "Form5",
    "Form6",
    "Form6Candidate",
    "Form7",
    "Form9",
    "Form9Custodian",
    "Form13",
    "Form24",
    "Form99",
]
