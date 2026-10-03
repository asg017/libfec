"""Generate ``covers.pyi`` and ``itemizations.pyi`` from `fec-parser`'s structs.

The classes in ``libfec_parser.covers`` *are* the Rust structs in
``crates/fec-parser/src/covers/`` (a ``#[pyclass]`` through ``cfg_attr``), so
their stubs are derived from that source rather than written by hand: every
``pub struct`` with its documented ``pub`` fields, plus the helper methods
``covers/python.rs`` exposes with ``cover_class!``.  The cover source follows
strict conventions (see ``covers/mod.rs``), which is what lets a small line
parser stand in for a Rust one.

    python scripts/gen_cover_stubs.py            # rewrite covers.pyi, itemizations.pyi
    python scripts/gen_cover_stubs.py --check    # exit 1 if it is stale

``tests/test_covers.py`` runs ``--check``, and ``make stubs`` runs stubtest
against the built extension, so neither the fields nor the methods can drift.
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
SRC = ROOT / "crates" / "fec-parser" / "src"
PKG = ROOT / "crates" / "fec-py" / "python" / "libfec_parser"

#: Rust types with a fixed Python spelling; any other name is a cover class.
SCALARS = {
    "String": "str",
    "&'static str": "str",
    "&str": "str",
    "bool": "bool",
    "f64": "float",
    "i16": "int",
    "u16": "int",
    "Date": "date",
    "jiff::civil::Date": "date",
    "Label": "str | None",
}

#: The ``CoverData`` union: the top-level form classes, one per `Cover` variant.
#: (The ``Itemization`` union is read from ``itemization_to_py``'s match arms.)
FORMS = [
    "Form1", "Form1M", "Form2", "Form3", "Form3L", "Form3P", "Form3X", "Form4",
    "Form5", "Form6", "Form7", "Form9", "Form13", "Form24", "Form99",
]


def py_type(rust: str) -> str:
    rust = rust.strip()
    for wrapper, fmt in (("Option<", "{} | None"), ("Vec<", "list[{}]"), ("Box<", "{}")):
        if rust.startswith(wrapper) and rust.endswith(">"):
            return fmt.format(py_type(rust[len(wrapper):-1]))
    return SCALARS.get(rust, rust)


def docstring(lines: list[str], indent: str) -> list[str]:
    """``///`` lines as a docstring body, or nothing."""
    if not lines:
        return []
    text = "\n".join(lines).strip().replace("\\", "\\\\").replace('"""', '\\"\\"\\"')
    body = text.replace("\n", "\n" + indent).replace(indent + "\n", "\n")
    return [f'{indent}"""{body}"""']


def parse_structs(src: Path) -> tuple[dict[str, tuple[list[str], list[tuple[str, str, list[str]]]]], dict[tuple[str, str], list[str]]]:
    """``{struct: (doc, [(field, rust type, doc)])}`` and ``{(type, method): doc}``."""
    structs: dict[str, tuple[list[str], list[tuple[str, str, list[str]]]]] = {}
    method_docs: dict[tuple[str, str], list[str]] = {}
    for path in sorted(src.glob("*.rs")):
        if path.name in ("fields.rs", "python.rs"):
            continue
        doc: list[str] = []
        current: str | None = None
        impl: str | None = None
        in_attribute = False
        pyclass = False
        for line in path.read_text().splitlines():
            stripped = line.strip()
            if in_attribute:  # the rest of a multi-line `#[cfg_attr(...)]`
                in_attribute = stripped != ")]"
                pyclass = pyclass or "pyclass" in stripped
                continue
            if stripped.startswith("///"):
                doc.append(stripped[4:] if stripped.startswith("/// ") else stripped[3:])
                continue
            if stripped.startswith("#["):
                in_attribute = not stripped.endswith("]")
                pyclass = pyclass or "pyclass" in stripped
                continue
            if m := re.match(r"pub struct (\w+) \{", stripped):
                # Only `#[pyclass]` structs; a plain helper struct is skipped.
                current, impl = (m.group(1) if pyclass else None), None
                if current:
                    structs[current] = (doc, [])
                pyclass = False
            elif m := re.match(r"impl (\w+) \{", stripped):
                current, impl = None, m.group(1)
            elif stripped == "}" and not line.startswith(" "):
                current = impl = None
            elif current and (m := re.match(r"pub (\w+): (.+),$", stripped)):
                structs[current][1].append((m.group(1), m.group(2), doc))
            elif impl and (m := re.match(r"pub fn (\w+)\(&self\)", stripped)):
                method_docs[(impl, m.group(1))] = doc
            doc = []
    return structs, method_docs


def parse_methods(src: Path) -> dict[str, list[tuple[str, str]]]:
    """``{struct: [(python name, rust return type)]}`` from ``cover_class!`` calls."""
    source = (src / "python.rs").read_text()
    methods: dict[str, list[tuple[str, str]]] = {}
    for call in re.finditer(r"^cover_class!\((.*?)\);", source, re.S | re.M):
        args = call.group(1).strip()
        name = re.match(r"\s*(\w+)", args).group(1)
        methods[name] = [
            (m.group(1), m.group(2).strip())
            for m in re.finditer(r'"(\w+)" \w+ = \w+ -> ([^,]+?)\s*(?:,|$)', args)
        ]
    return methods


def union_members(src: Path) -> list[str]:
    """The ``Itemization`` union: one class per ``itemization_to_py`` match arm."""
    return re.findall(r"Itemization::(\w+)\(\w+\) =>", (src / "python.rs").read_text())


#: ``(module, source dir, union name, union members, extra imports)``.
TARGETS = [
    ("covers", SRC / "covers", "CoverData", lambda: FORMS, []),
    (
        "itemizations",
        SRC / "itemizations",
        "Itemization",
        lambda: union_members(SRC / "itemizations"),
        ["from .covers import Address, PersonName"],
    ),
]


def render(module: str, src: Path, union: str, members: list[str], imports: list[str]) -> str:
    structs, method_docs = parse_structs(src)
    methods = parse_methods(src)
    missing = set(structs) ^ set(methods)
    if missing:
        raise SystemExit(f"structs and cover_class! calls disagree: {sorted(missing)}")

    out = [
        f'"""Type stubs for :mod:`libfec_parser.{module}`.',
        "",
        "Generated by ``crates/fec-py/scripts/gen_cover_stubs.py`` from",
        f"``crates/fec-parser/src/{module}/*.rs`` -- do not edit by hand.",
        '"""',
        "",
        "from datetime import date",
        "from typing import Any, TypeAlias, final",
        *imports,
        "",
        "__all__ = [",
        *[f'    "{name}",' for name in [union, *structs]],
        "]",
        "",
        f"{union}: TypeAlias = (",
        "    " + " | ".join(members),
        ")",
    ]
    for name, (doc, fields) in structs.items():
        out += ["", "@final", f"class {name}:"]
        out += docstring(doc, "    ")
        for field, rust, field_doc in fields:
            out += ["    @property", f"    def {field}(self) -> {py_type(rust)}:"]
            out += docstring(field_doc, "        ") or ["        ..."]
        for method, ret in methods[name]:
            out.append(f"    def {method}(self) -> {py_type(ret)}:")
            out += docstring(method_docs.get((name, method), []), "        ") or ["        ..."]
        out += [
            "    def to_dict(self) -> dict[str, Any]:",
            '        """The fields as a `dict`, nested covers as nested dicts."""',
            "    def __eq__(self, other: object, /) -> bool: ...",
        ]
    return "\n".join(out) + "\n"


def main() -> int:
    stale = False
    for module, src, union, members, imports in TARGETS:
        out = PKG / f"{module}.pyi"
        text = render(module, src, union, members(), imports)
        if "--check" in sys.argv[1:]:
            if not out.exists() or out.read_text() != text:
                print(f"{out} is stale; run: python {Path(__file__).name}", file=sys.stderr)
                stale = True
        else:
            out.write_text(text)
    return 1 if stale else 0


if __name__ == "__main__":
    sys.exit(main())
