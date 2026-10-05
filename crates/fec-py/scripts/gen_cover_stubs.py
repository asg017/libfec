"""Generate ``python/libfec_parser/covers.pyi`` from `fec-parser`'s cover structs.

The classes in ``libfec_parser.covers`` *are* the Rust structs in
``crates/fec-parser/src/covers/`` (a ``#[pyclass]`` through ``cfg_attr``), so
their stubs are derived from that source rather than written by hand: every
``pub struct`` with its documented ``pub`` fields, plus the helper methods
``covers/python.rs`` exposes with ``cover_class!``.  The cover source follows
strict conventions (see ``covers/mod.rs``), which is what lets a small line
parser stand in for a Rust one.

    python scripts/gen_cover_stubs.py            # rewrite covers.pyi
    python scripts/gen_cover_stubs.py --check    # exit 1 if it is stale

``tests/test_covers.py`` runs ``--check``, and ``make stubs`` runs stubtest
against the built extension, so neither the fields nor the methods can drift.
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
COVERS = ROOT / "crates" / "fec-parser" / "src" / "covers"
OUT = ROOT / "crates" / "fec-py" / "python" / "libfec_parser" / "covers.pyi"

#: Rust types with a fixed Python spelling; any other name is a cover class.
SCALARS = {
    "String": "str",
    "&'static str": "str",
    "bool": "bool",
    "f64": "float",
    "i16": "int",
    "u16": "int",
    "Date": "date",
    "jiff::civil::Date": "date",
    "Label": "str | None",
}

#: The ``CoverData`` union: the top-level form classes, one per `Cover` variant.
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


def parse_structs() -> tuple[dict[str, tuple[list[str], list[tuple[str, str, list[str]]]]], dict[tuple[str, str], list[str]]]:
    """``{struct: (doc, [(field, rust type, doc)])}`` and ``{(type, method): doc}``."""
    structs: dict[str, tuple[list[str], list[tuple[str, str, list[str]]]]] = {}
    method_docs: dict[tuple[str, str], list[str]] = {}
    for path in sorted(COVERS.glob("*.rs")):
        if path.name in ("fields.rs", "python.rs"):
            continue
        doc: list[str] = []
        current: str | None = None
        impl: str | None = None
        in_attribute = False
        for line in path.read_text(encoding="utf-8").splitlines():
            stripped = line.strip()
            if in_attribute:  # the rest of a multi-line `#[cfg_attr(...)]`
                in_attribute = stripped != ")]"
                continue
            if stripped.startswith("///"):
                doc.append(stripped[4:] if stripped.startswith("/// ") else stripped[3:])
                continue
            if stripped.startswith("#["):
                in_attribute = not stripped.endswith("]")
                continue
            if m := re.match(r"pub struct (\w+) \{", stripped):
                current, impl = m.group(1), None
                structs[current] = (doc, [])
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


def parse_methods() -> dict[str, list[tuple[str, str]]]:
    """``{struct: [(python name, rust return type)]}`` from ``cover_class!`` calls."""
    source = (COVERS / "python.rs").read_text(encoding="utf-8")
    methods: dict[str, list[tuple[str, str]]] = {}
    for call in re.finditer(r"^cover_class!\((.*?)\);", source, re.S | re.M):
        args = call.group(1).strip()
        name = re.match(r"\s*(\w+)", args).group(1)
        methods[name] = [
            (m.group(1), m.group(2).strip())
            for m in re.finditer(r'"(\w+)" \w+ = \w+ -> ([^,]+?)\s*(?:,|$)', args)
        ]
    return methods


def render() -> str:
    structs, method_docs = parse_structs()
    methods = parse_methods()
    missing = set(structs) ^ set(methods)
    if missing:
        raise SystemExit(f"structs and cover_class! calls disagree: {sorted(missing)}")

    out = [
        '"""Type stubs for :mod:`libfec_parser.covers`.',
        "",
        "Generated by ``crates/fec-py/scripts/gen_cover_stubs.py`` from",
        "``crates/fec-parser/src/covers/*.rs`` -- do not edit by hand.",
        '"""',
        "",
        "from datetime import date",
        "from typing import Any, TypeAlias, final",
        "",
        "__all__ = [",
        *[f'    "{name}",' for name in ["CoverData", *structs]],
        "]",
        "",
        "CoverData: TypeAlias = (",
        "    " + " | ".join(FORMS),
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
    text = render()
    if "--check" in sys.argv[1:]:
        if OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT} is stale; run: python {Path(__file__).name}", file=sys.stderr)
            return 1
        return 0
    OUT.write_text(text, encoding="utf-8", newline="\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
