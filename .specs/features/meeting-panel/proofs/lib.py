"""Shared helpers for the meeting-panel proof scripts: production source only (test modules cut)."""
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
DESKTOP = ROOT / "apps/desktop/src"


def production(path: pathlib.Path) -> str:
    """The file up to its first `#[cfg(test)]`; every test module in these files is at the end."""
    text = path.read_text()
    cut = text.find("#[cfg(test)]")
    return text if cut < 0 else text[:cut]


def fn_body(source: str, signature: str) -> str:
    """The body of the first function whose header contains `signature` (brace matched)."""
    start = source.find(signature)
    if start < 0:
        fail(f"function not found: {signature}")
    open_at = source.index("{", start)
    depth = 0
    for i in range(open_at, len(source)):
        if source[i] == "{":
            depth += 1
        elif source[i] == "}":
            depth -= 1
            if depth == 0:
                return source[open_at : i + 1]
    fail(f"unbalanced body: {signature}")


def occurrences(pattern: str):
    """(relative path, line) of every production match of `pattern` under apps/desktop/src."""
    hits = []
    for path in sorted(DESKTOP.rglob("*.rs")):
        for line in production(path).splitlines():
            if line.strip().startswith("//"):
                continue
            if re.search(pattern, line):
                hits.append((str(path.relative_to(ROOT)), line.strip()))
    return hits


def fail(message: str):
    print(f"FAIL: {message}")
    sys.exit(1)


def ok(message: str):
    print(f"ok: {message}")
