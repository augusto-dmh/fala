"""Exporta o histórico de texto do Wispr Flow para o corpus do `fala-cli bench format`.

Uso: python -I scripts/export-wispr-corpus.py <flow.sqlite> <saida.jsonl>
     python -I scripts/export-wispr-corpus.py --self-test

Abre o banco só para leitura (`mode=ro&immutable=1`: nem o WAL nem o lock do app são tocados) e
escreve uma linha JSON por ditado que tem texto bruto e formatado, só com os campos de texto:
{id, raw, formatted, pasted, edited, app, lang, words}. Nada de áudio, tela, contexto, token,
e-mail ou metadado de conta. O corpus tem texto pessoal: fica fora do repo, e este script não
imprime nada dele, só a contagem.
"""

import json
import pathlib
import sqlite3
import sys
import tempfile

FIELDS = ("id", "raw", "formatted", "pasted", "edited", "app", "lang", "words")

QUERY = """
SELECT transcriptEntityId, asrText, formattedText, pastedText, editedText, app,
       COALESCE(detectedLanguage, language), numWords
FROM History
WHERE asrText IS NOT NULL AND asrText != ''
  AND formattedText IS NOT NULL AND formattedText != ''
ORDER BY timestamp, transcriptEntityId
"""


def connect_readonly(path):
    uri = pathlib.Path(path).resolve().as_uri() + "?mode=ro&immutable=1"
    return sqlite3.connect(uri, uri=True)


def pending_journals(path):
    """`-wal`/`-journal` ao lado do banco: o `immutable=1` não os lê, então o app pode estar
    escrevendo e as linhas mais novas podem faltar ou vir pela metade."""
    db = pathlib.Path(path)
    found = []
    for suffix in ("-wal", "-journal"):
        journal = db.with_name(db.name + suffix)
        if journal.exists() and journal.stat().st_size > 0:
            found.append(journal.name)
    return found


def blank_to_none(value):
    return value if value else None


def export(db_path, out_path):
    conn = connect_readonly(db_path)
    try:
        rows = conn.execute(QUERY).fetchall()
    finally:
        conn.close()
    with open(out_path, "w", encoding="utf-8", newline="\n") as out:
        for row in rows:
            id_, raw, formatted, pasted, edited, app, lang, words = row
            record = {
                "id": id_,
                "raw": raw,
                "formatted": formatted,
                "pasted": blank_to_none(pasted),
                "edited": blank_to_none(edited),
                "app": blank_to_none(app),
                "lang": blank_to_none(lang),
                "words": words if words is not None else len(raw.split()),
            }
            out.write(json.dumps(record, ensure_ascii=False) + "\n")
    return len(rows)


def self_test():
    with tempfile.TemporaryDirectory() as tmp:
        db = pathlib.Path(tmp) / "flow.sqlite"
        conn = sqlite3.connect(db)
        conn.executescript(
            """
            CREATE TABLE History (
                transcriptEntityId TEXT, asrText TEXT, formattedText TEXT, editedText TEXT,
                pastedText TEXT, timestamp TEXT, app TEXT, language TEXT,
                detectedLanguage TEXT, numWords INTEGER, additionalContext TEXT, audio BLOB
            );
            INSERT INTO History VALUES
                ('b', 'um dois', 'Um, dois.', 'Um e dois.', 'Um, dois.', '2026-01-02',
                 'Slack', NULL, 'pt', 2, '{"email":"a@b.c"}', x'00'),
                ('a', 'hello there', 'Hello there.', '', 'Hello there.', '2026-01-01',
                 'Cursor', 'en', NULL, NULL, '{"token":"segredo"}', NULL),
                ('c', 'sem formatado', NULL, NULL, NULL, '2026-01-03', 'Slack', NULL, 'pt', 2,
                 NULL, NULL);
            """
        )
        conn.commit()
        conn.close()
        out = pathlib.Path(tmp) / "corpus.jsonl"
        assert pending_journals(db) == []
        (pathlib.Path(tmp) / "flow.sqlite-wal").write_bytes(b"x")
        assert pending_journals(db) == ["flow.sqlite-wal"]
        (pathlib.Path(tmp) / "flow.sqlite-wal").unlink()
        assert export(db, out) == 2
        lines = out.read_text(encoding="utf-8").splitlines()
        records = [json.loads(line) for line in lines]
        assert [r["id"] for r in records] == ["a", "b"], records
        for record in records:
            assert tuple(record) == FIELDS, record
        assert records[0]["edited"] is None
        assert records[0]["lang"] == "en"
        assert records[0]["words"] == 2
        assert records[1]["edited"] == "Um e dois."
        assert records[1]["lang"] == "pt"
        text = out.read_text(encoding="utf-8")
        assert "a@b.c" not in text and "segredo" not in text
        ro = connect_readonly(db)
        try:
            ro.execute("CREATE TABLE x (a)")
        except sqlite3.OperationalError:
            pass
        else:
            raise AssertionError("o banco abriu com escrita")
        finally:
            ro.close()
    print("self-test ok", file=sys.stderr)


def main(argv):
    if argv == ["--self-test"]:
        self_test()
        return 0
    if len(argv) != 2:
        print(__doc__.strip().splitlines()[2], file=sys.stderr)
        return 2
    for journal in pending_journals(argv[0]):
        print(
            f"aviso: {journal} existe; feche o app e exporte de novo para não perder linhas",
            file=sys.stderr,
        )
    count = export(argv[0], argv[1])
    print(f"{count} ditados exportados", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
