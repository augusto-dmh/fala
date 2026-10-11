#!/usr/bin/env python3
"""Guards what ADR-0008 decides about distribution: docs/RELEASE.md exists and says what it must,
the NSIS installer is explicit and unsigned, and the updater stays off. Also guards that the
version has one source, `[workspace.package]` of the root Cargo.toml.

Run from the repo root: python3 scripts/ci/test_release_config.py [-k <test name>]
Standard library only, so the CI job needs no pip install.
"""

import copy
import json
import re
import tomllib
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
ROOT_CARGO = ROOT / "Cargo.toml"
PACKAGE_JSON = ROOT / "package.json"
CONF = ROOT / "apps" / "desktop" / "tauri.conf.json"
RELEASE_MD = ROOT / "docs" / "RELEASE.md"
README = ROOT / "README.md"
CI_YML = ROOT / ".github" / "workflows" / "ci.yml"

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


def updater_problems(c: dict) -> list[str]:
    """Everything in a Tauri config that would switch the updater on."""
    problems = []
    updater = c.get("plugins", {}).get("updater", {})
    if updater.get("pubkey", "") != "":
        problems.append(f"plugins.updater.pubkey is {updater['pubkey']!r}, want ''")
    if updater.get("endpoints", []) != []:
        problems.append(f"plugins.updater.endpoints is {updater['endpoints']!r}, want []")
    if c["bundle"].get("createUpdaterArtifacts") is not False:
        problems.append(f"bundle.createUpdaterArtifacts is {c['bundle'].get('createUpdaterArtifacts')!r}, want False")
    return problems


def toml(path: Path) -> dict:
    return tomllib.loads(path.read_text(encoding="utf-8"))


def member_manifests() -> dict[str, dict]:
    """Every Cargo.toml of the workspace, keyed by its path relative to the repo root."""
    manifests = {}
    for pattern in toml(ROOT_CARGO)["workspace"]["members"]:
        for member in sorted(ROOT.glob(pattern)):
            manifest = member / "Cargo.toml"
            if manifest.is_file():
                manifests[manifest.relative_to(ROOT).as_posix()] = toml(manifest)
    return manifests


def tauri_confs() -> dict[str, dict]:
    return {p.name: json.loads(p.read_text(encoding="utf-8")) for p in sorted(CONF.parent.glob("tauri*.conf.json"))}


def version_problems(root: dict, members: dict[str, dict], confs: dict[str, dict], package: dict) -> list[str]:
    """Everything that would give the app a version other than `[workspace.package]` of the root Cargo.toml.

    Tauri reads `version` from the Tauri config when it is there and falls back to the app's
    Cargo.toml otherwise, for both the app (`package_info`) and the installer name.
    """
    version = root.get("workspace", {}).get("package", {}).get("version")
    if not version:
        return ["[workspace.package] of Cargo.toml has no version"]
    problems = []
    for path, manifest in members.items():
        got = manifest.get("package", {}).get("version")
        if got != {"workspace": True}:
            problems.append(f"{path} has package.version {got!r}, want version.workspace = true")
    for name, c in confs.items():
        if "version" in c:
            problems.append(f"apps/desktop/{name} sets version {c['version']!r}; the version comes from Cargo.toml")
    if package.get("version") != version:
        problems.append(f"package.json has version {package.get('version')!r}, want {version!r} from Cargo.toml")
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

    def test_updater_stays_off(self):
        self.assertEqual(updater_problems(conf()), [])

    def test_guard_rejects_enabled_updater(self):
        base = conf()
        with_key = copy.deepcopy(base)
        with_key["plugins"]["updater"]["pubkey"] = "dW50cnVzdGVkIGNvbW1lbnQ6IG5vdCBhIHJlYWwga2V5"
        with_endpoint = copy.deepcopy(base)
        with_endpoint["plugins"]["updater"]["endpoints"] = ["https://example.invalid/latest.json"]
        with_artifacts = copy.deepcopy(base)
        with_artifacts["bundle"]["createUpdaterArtifacts"] = True
        for name, mutated in (("pubkey", with_key), ("endpoint", with_endpoint), ("artifacts", with_artifacts)):
            self.assertTrue(updater_problems(mutated), f"guard accepted an enabled updater ({name})")

    def test_identity_unchanged(self):
        c = conf()
        self.assertEqual(c["productName"], "Fala")
        self.assertEqual(c["identifier"], "br.com.augusto.fala")
        self.assertIn("nsis", c["bundle"]["targets"])


class VersionSource(unittest.TestCase):
    def actual(self) -> tuple[dict, dict[str, dict], dict[str, dict], dict]:
        return toml(ROOT_CARGO), member_manifests(), tauri_confs(), json.loads(PACKAGE_JSON.read_text(encoding="utf-8"))

    def test_one_version_source(self):
        self.assertEqual(version_problems(*self.actual()), [])

    def test_every_member_is_checked(self):
        members = member_manifests()
        for path in ("apps/desktop/Cargo.toml", "apps/cli/Cargo.toml", "crates/core/Cargo.toml"):
            self.assertIn(path, members)

    def test_guard_rejects_a_second_source(self):
        root, members, confs, package = self.actual()
        with_conf = copy.deepcopy(confs)
        with_conf["tauri.conf.json"]["version"] = root["workspace"]["package"]["version"]
        with_crate = copy.deepcopy(members)
        with_crate["apps/desktop/Cargo.toml"]["package"]["version"] = root["workspace"]["package"]["version"]
        with_package = copy.deepcopy(package)
        with_package["version"] = "9.9.9"
        without_package = copy.deepcopy(package)
        del without_package["version"]
        without_root = copy.deepcopy(root)
        del without_root["workspace"]["package"]["version"]
        cases = {
            "tauri.conf.json version": (root, members, with_conf, package),
            "literal crate version": (root, with_crate, confs, package),
            "package.json diverges": (root, members, confs, with_package),
            "package.json without version": (root, members, confs, without_package),
            "no workspace version": (without_root, members, confs, package),
        }
        for name, args in cases.items():
            self.assertTrue(version_problems(*args), f"guard accepted {name}")

    def test_release_md_names_the_source(self):
        body = release_sections()["Versão e changelog"]
        for needle in ("[workspace.package]", "cargo update --workspace", "test_release_config.py"):
            self.assertIn(needle, body, f"'Versão e changelog' lacks {needle!r}")


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
