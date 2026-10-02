#!/usr/bin/env python3
"""Guards what ADR-0008 decides about distribution: docs/RELEASE.md exists and says what it must,
the NSIS installer is explicit and unsigned, and the updater stays off.

Run from the repo root: python3 scripts/ci/test_release_config.py [-k <test name>]
Standard library only, so the CI job needs no pip install.
"""

import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
RELEASE_MD = ROOT / "docs" / "RELEASE.md"
README = ROOT / "README.md"

SECTIONS = [
    "Antes de começar",
    "Versão e changelog",
    "Construir o instalador",
    "Sem assinatura: o aviso do SmartScreen",
    "Validar no Windows",
    "Publicar",
    "Updater",
    "Retirar uma versão",
]


def release_sections() -> dict[str, str]:
    """Maps each `## ` heading of docs/RELEASE.md to its body, in file order.

    Headings inside fenced code blocks are not sections; a repeated heading is an error.
    """
    sections: dict[str, list[str]] = {}
    current = None
    in_fence = False
    for line in RELEASE_MD.read_text().splitlines():
        if line.startswith("```"):
            in_fence = not in_fence
        heading = None if in_fence else re.fullmatch(r"## (.+)", line)
        if heading:
            current = heading.group(1)
            if current in sections:
                raise AssertionError(f"docs/RELEASE.md repeats the section {current!r}")
            sections[current] = []
        elif current is not None:
            sections[current].append(line)
    return {title: "\n".join(body) for title, body in sections.items()}


class ReleaseRunbook(unittest.TestCase):
    def assert_contains_all(self, section: str, needles: list[str]):
        body = release_sections()[section]
        for needle in needles:
            self.assertIn(needle, body, f"'{section}' lacks {needle!r}")

    def test_release_md_sections_in_order(self):
        self.assertEqual(list(release_sections()), SECTIONS)

    def test_release_md_build_command(self):
        self.assert_contains_all(
            "Construir o instalador",
            [
                "bun run tauri build --bundles nsis",
                r"target\release\bundle\nsis\Fala_<versão>_x64-setup.exe",
            ],
        )

    def test_release_md_marks_windows_only_steps(self):
        self.assertIn("TODO(windows)", release_sections()["Validar no Windows"])

    def test_release_md_updater_section(self):
        self.assert_contains_all(
            "Updater",
            ["desligado", "plugins.updater", "createUpdaterArtifacts", "ADR-0008", "nunca no repositório"],
        )

    def test_release_md_version_section(self):
        self.assert_contains_all(
            "Versão e changelog", ["mantenedor", "CHANGELOG.md", "chore(release): vX.Y.Z"]
        )

    def test_smartscreen_steps_in_release_md_and_readme(self):
        steps = ["Mais informações", "Executar assim mesmo"]
        self.assert_contains_all("Sem assinatura: o aviso do SmartScreen", steps + ["README.md"])
        readme = README.read_text()
        for step in steps:
            self.assertIn(step, readme, f"README.md lacks {step!r}")


if __name__ == "__main__":
    unittest.main()
