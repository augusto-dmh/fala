#!/usr/bin/env python3
"""Guards what ADR-0008 decides about distribution (docs/RELEASE.md exists and says what it must,
the NSIS installer is explicit and unsigned) and what ADR-0018 decides about the updater (on, with
a minisign key and the GitHub Release feed).

Run from the repo root: python3 scripts/ci/test_release_config.py [-k <test name>]
Standard library only, so the CI job needs no pip install.
"""

import base64
import copy
import json
import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CONF = ROOT / "apps" / "desktop" / "tauri.conf.json"
RELEASE_MD = ROOT / "docs" / "RELEASE.md"
README = ROOT / "README.md"
CI_YML = ROOT / ".github" / "workflows" / "ci.yml"
KEY_SCRIPT = ROOT / "scripts" / "release" / "new-updater-key.ps1"
FEED = "https://github.com/augusto-dmh/fala/releases/latest/download/latest.json"
# A well-formed minisign public key that verifies nothing: "Ed" and 40 zero bytes (key id + key).
FAKE_PUBKEY = base64.b64encode(
    b"untrusted comment: minisign public key\n" + base64.b64encode(b"Ed" + bytes(40)) + b"\n"
).decode()

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


def conf() -> dict:
    return json.loads(CONF.read_text())


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


def is_minisign_pubkey(value: str) -> bool:
    """The base64 of a minisign .pub file, as `tauri signer generate` writes it."""
    try:
        lines = base64.b64decode(value, validate=True).decode("utf-8").splitlines()
        raw = base64.b64decode(lines[1], validate=True)
    except (ValueError, IndexError):
        return False
    return lines[0].startswith("untrusted comment:") and len(raw) == 42 and raw[:2] == b"Ed"


def updater_problems(c: dict) -> list[str]:
    """Everything in a Tauri config that keeps the updater from working as ADR-0018 decides."""
    problems = []
    updater = c.get("plugins", {}).get("updater", {})
    if not is_minisign_pubkey(updater.get("pubkey", "")):
        problems.append(
            f"plugins.updater.pubkey is {updater.get('pubkey')!r}, not a minisign public key; "
            "the maintainer runs scripts/release/new-updater-key.ps1"
        )
    if updater.get("endpoints") != [FEED]:
        problems.append(f"plugins.updater.endpoints is {updater.get('endpoints')!r}, want [{FEED!r}]")
    if c["bundle"].get("createUpdaterArtifacts") is not True:
        problems.append(f"bundle.createUpdaterArtifacts is {c['bundle'].get('createUpdaterArtifacts')!r}, want True")
    return problems


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


class BundleConfig(unittest.TestCase):
    def test_bundle_targets(self):
        self.assertEqual(conf()["bundle"]["targets"], ["nsis", "deb", "rpm", "appimage"])

    def test_nsis_config(self):
        nsis = conf()["bundle"]["windows"]["nsis"]
        self.assertEqual(nsis.get("installMode"), "currentUser")
        self.assertEqual(nsis.get("languages"), ["PortugueseBR", "English"])
        self.assertIs(nsis.get("displayLanguageSelector"), False)
        self.assertEqual(nsis.get("template"), "nsis/installer.nsi")

    def test_windows_unsigned_with_bootstrapper(self):
        windows = conf()["bundle"]["windows"]
        self.assertEqual(windows.get("webviewInstallMode"), {"type": "downloadBootstrapper"})
        for field in ("certificateThumbprint", "signCommand", "tsp"):
            self.assertNotIn(field, windows, f"bundle.windows.{field} would sign the installer")

    def test_updater_on(self):
        self.assertEqual(updater_problems(conf()), [])

    def test_guard_accepts_a_real_key_and_rejects_the_rest(self):
        good = copy.deepcopy(conf())
        good["plugins"]["updater"]["pubkey"] = FAKE_PUBKEY
        self.assertEqual(updater_problems(good), [])
        mutations = {
            "placeholder": ("pubkey", "PLACEHOLDER: run scripts/release/new-updater-key.ps1"),
            "empty key": ("pubkey", ""),
            "no endpoint": ("endpoints", []),
            "other endpoint": ("endpoints", ["https://example.invalid/latest.json"]),
            "two endpoints": ("endpoints", [FEED, "https://example.invalid/latest.json"]),
        }
        for name, (field, value) in mutations.items():
            mutated = copy.deepcopy(good)
            mutated["plugins"]["updater"][field] = value
            self.assertTrue(updater_problems(mutated), f"guard accepted {name}")
        no_artifacts = copy.deepcopy(good)
        no_artifacts["bundle"]["createUpdaterArtifacts"] = False
        self.assertTrue(updater_problems(no_artifacts), "guard accepted createUpdaterArtifacts false")

    def test_updater_switches_agree(self):
        rust = (ROOT / "apps" / "desktop" / "src" / "settings.rs").read_text(encoding="utf-8")
        ts = (ROOT / "src" / "lib" / "updater.ts").read_text(encoding="utf-8")
        self.assertIn("pub const UPDATER_ENABLED: bool = true;", rust)
        self.assertIn("export const UPDATER_ENABLED = true;", ts)

    def test_key_script_writes_only_the_public_key(self):
        script = KEY_SCRIPT.read_text(encoding="utf-8")
        self.assertIn("tauri signer generate -w $keyPath", script)
        self.assertIn("Get-Content $pubPath", script)
        self.assertNotIn("Get-Content $keyPath", script, "the script must never read the private key")
        self.assertIn("already exists", script, "the script must refuse to overwrite a key")

    def test_identity_unchanged(self):
        c = conf()
        self.assertEqual(c["productName"], "Fala")
        self.assertEqual(c["identifier"], "br.com.augusto.fala")
        self.assertIn("nsis", c["bundle"]["targets"])


class CiWiring(unittest.TestCase):
    def test_ci_runs_release_config_test(self):
        text = CI_YML.read_text()
        job = re.search(r"^  rust:\n(.*?)(?=^  [a-z][a-z-]*:\n)", text, re.DOTALL | re.MULTILINE)
        self.assertIsNotNone(job, "no rust job in ci.yml")
        steps = [s for s in re.split(r"^      - ", job.group(1), flags=re.MULTILINE) if s.strip()]
        hits = [s for s in steps if re.search(r"^\s*run: python3 scripts/ci/test_release_config\.py\s*$", s, re.MULTILINE)]
        self.assertEqual(len(hits), 1, "the rust job needs exactly one step running this test")
        self.assertNotIn("continue-on-error", hits[0])


if __name__ == "__main__":
    unittest.main()
