"""C34: no log above debug in the desktop meeting module formats typed notes, transcript text,
notes Markdown, titles or keys."""
import re

from lib import DESKTOP, fail, ok, production

FORBIDDEN = re.compile(r"annotations|\btext\b|markdown|notes_md|title|expose|\bkey\b|segment\.text")
files = sorted((DESKTOP / "meeting").glob("*.rs")) + [DESKTOP / "commands/meeting.rs"]
calls = 0
for path in files:
    source = production(path)
    for match in re.finditer(r"log::(info|warn|error|trace)!\(", source):
        end = source.find(");", match.end())
        call = source[match.start() : end]
        calls += 1
        literal = re.search(r'"((?:[^"\\]|\\.)*)"', call)
        if not literal:
            fail(f"{path.name}: log without a literal: {call!r}")
        placeholders = " ".join(re.findall(r"\{([^}]*)\}", literal.group(1)))
        arguments = call[literal.end() :]
        if FORBIDDEN.search(placeholders) or FORBIDDEN.search(arguments):
            fail(f"{path.name}: {call!r}")
if calls < 10:
    fail(f"only {calls} log calls found; the module moved?")
ok(f"{calls} log calls above debug carry ids, counts and errors only")
