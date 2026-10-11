#!/usr/bin/env python3
"""Tests scripts/release/gen-latest-json.py with a throwaway key: signs a fake installer the way
`tauri signer sign` does (minisign, prehashed) and checks the manifest and every refusal.

Run from the repo root: python3 scripts/ci/test_gen_latest_json.py
Standard library only.
"""

import base64
import hashlib
import importlib.util
import json
import shutil
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("gen", ROOT / "scripts" / "release" / "gen-latest-json.py")
gen = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(gen)


def _compress(point):
    zinv = pow(point[2], gen.P - 2, gen.P)
    x, y = point[0] * zinv % gen.P, point[1] * zinv % gen.P
    return int.to_bytes(y | ((x & 1) << 255), 32, "little")


class Key:
    """An Ed25519 key in minisign's layout, signing as RFC 8032 section 6 does."""

    def __init__(self, seed: bytes, key_id: bytes):
        h = hashlib.sha512(seed).digest()
        self.a = (int.from_bytes(h[:32], "little") & ((1 << 254) - 8)) | (1 << 254)
        self.prefix = h[32:]
        self.public = _compress(gen._mul(self.a, gen.G))
        self.key_id = key_id

    def sign(self, message: bytes) -> bytes:
        r = gen._sha512_modq(self.prefix + message)
        rs = _compress(gen._mul(r, gen.G))
        s = (r + gen._sha512_modq(rs + self.public + message) * self.a) % gen.Q
        return rs + int.to_bytes(s, 32, "little")

    def pubkey(self) -> str:
        """The string `tauri signer generate` prints and plugins.updater.pubkey holds."""
        raw = base64.b64encode(b"Ed" + self.key_id + self.public).decode()
        return base64.b64encode(f"untrusted comment: minisign public key\n{raw}\n".encode()).decode()

    def sig(self, data: bytes, file_name: str) -> str:
        """A Tauri .sig: base64 of the minisign signature file."""
        signature = self.sign(hashlib.blake2b(data).digest())
        comment = f"timestamp:1760000000\tfile:{file_name}"
        global_sig = self.sign(signature + comment.encode())
        text = (
            "untrusted comment: signature from tauri secret key\n"
            f"{base64.b64encode(b'ED' + self.key_id + signature).decode()}\n"
            f"trusted comment: {comment}\n"
            f"{base64.b64encode(global_sig).decode()}\n"
        )
        return base64.b64encode(text.encode()).decode()


KEY = Key(b"\x01" * 32, b"\x11" * 8)
OTHER = Key(b"\x02" * 32, b"\x22" * 8)
VERSION = "0.1.0"
NAME = f"Fala_{VERSION}_x64-setup.exe"
DATE = "2026-10-11T12:00:00Z"


class GenLatestJson(unittest.TestCase):
    def setUp(self):
        self.dir = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.dir)
        self.exe = self.dir / NAME
        self.exe.write_bytes(b"MZ fake installer " * 100)

    def write_sig(self, key=KEY, data=None, file_name=NAME):
        sig = self.exe.with_name(NAME + ".sig")
        sig.write_text(key.sig(data or self.exe.read_bytes(), file_name))
        return sig.read_text()

    def build(self, version=VERSION, exe=None, notes="## 0.1.0\n\n- first release"):
        return gen.build_manifest(version, exe or self.exe, notes, KEY.pubkey(), DATE)

    def assert_refused(self, needle, **kwargs):
        with self.assertRaises(gen.ManifestError) as caught:
            self.build(**kwargs)
        self.assertIn(needle, str(caught.exception))

    def test_ed25519_matches_rfc8032_vector(self):
        # RFC 8032 section 7.1, TEST 1.
        public = bytes.fromhex("d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a")
        signature = bytes.fromhex(
            "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e06522490155"
            "5fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b"
        )
        self.assertTrue(gen.ed25519_verify(public, b"", signature))
        self.assertFalse(gen.ed25519_verify(public, b"x", signature))

    def test_builds_both_platform_keys(self):
        sig = self.write_sig()
        manifest = self.build()
        url = f"https://github.com/augusto-dmh/fala/releases/download/v{VERSION}/{NAME}"
        self.assertEqual(manifest["version"], VERSION)
        self.assertEqual(manifest["notes"], "## 0.1.0\n\n- first release")
        self.assertEqual(manifest["pub_date"], DATE)
        self.assertEqual(sorted(manifest["platforms"]), ["windows-x86_64", "windows-x86_64-nsis"])
        for entry in manifest["platforms"].values():
            self.assertEqual(entry, {"signature": sig, "url": url})

    def test_rc_version(self):
        rc = "0.2.0-rc.1"
        exe = self.dir / f"Fala_{rc}_x64-setup.exe"
        exe.write_bytes(b"rc")
        exe.with_name(exe.name + ".sig").write_text(KEY.sig(b"rc", exe.name))
        self.assertEqual(self.build(version=rc, exe=exe)["version"], rc)

    def test_refuses_missing_sig(self):
        self.assert_refused(".sig is missing")

    def test_refuses_installer_of_another_version(self):
        self.write_sig()
        self.assert_refused("needs 'Fala_0.1.1_x64-setup.exe'", version="0.1.1")

    def test_refuses_bad_version(self):
        self.assert_refused("is not X.Y.Z", version="v0.1.0")

    def test_refuses_other_key(self):
        self.write_sig(key=OTHER)
        self.assert_refused("was made with key")

    def test_refuses_tampered_installer(self):
        self.write_sig(data=b"the bytes that were signed")
        self.assert_refused("does not verify against plugins.updater.pubkey")

    def test_refuses_sig_of_another_file(self):
        self.write_sig(file_name="Fala_0.0.9_x64-setup.exe")
        self.assert_refused("the .sig signs 'Fala_0.0.9_x64-setup.exe'")

    def test_refuses_empty_notes(self):
        self.write_sig()
        self.assert_refused("the notes are empty", notes="  \n")

    def test_refuses_placeholder_pubkey(self):
        with self.assertRaises(gen.ManifestError):
            gen.parse_pubkey("RUN scripts/release/new-updater-key.ps1")

    def test_cli_writes_nothing_on_refusal(self):
        conf = self.dir / "tauri.conf.json"
        conf.write_text(json.dumps({"plugins": {"updater": {"pubkey": KEY.pubkey()}}}))
        notes = self.dir / "notes.md"
        notes.write_text("- first release\n")
        out = self.dir / "latest.json"
        args = ["--version", VERSION, "--exe", str(self.exe), "--notes-file", str(notes),
                "--output", str(out), "--conf", str(conf), "--pub-date", DATE]
        self.assertEqual(gen.main(args), 1)
        self.assertFalse(out.exists())
        self.write_sig()
        self.assertEqual(gen.main(args), 0)
        self.assertEqual(json.loads(out.read_text())["version"], VERSION)


if __name__ == "__main__":
    unittest.main()
