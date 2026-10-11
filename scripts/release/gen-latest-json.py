#!/usr/bin/env python3
"""Builds the updater manifest (latest.json) for one Windows release and checks it first.

The manifest is attached to the GitHub Release next to the installer and its .sig, and
installed copies read it from releases/latest/download/latest.json (ADR-0018). Refuses an
installer named for another version, a missing .sig and a signature that does not verify
against plugins.updater.pubkey, so a broken manifest never reaches the draft.

Usage (repo root, after `bun run tauri build --bundles nsis` with the signing key in the session):
  python scripts/release/gen-latest-json.py --version 0.1.0 \
    --exe C:\\f\\release\\bundle\\nsis\\Fala_0.1.0_x64-setup.exe \
    --notes-file notas.md --output latest.json

Standard library only. The Ed25519 check follows RFC 8032 section 6; minisign signs the
BLAKE2b-512 hash of the file ("ED") and then the signature plus the trusted comment.
"""

import argparse
import base64
import hashlib
import json
import re
import sys
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CONF = ROOT / "apps" / "desktop" / "tauri.conf.json"
REPO = "augusto-dmh/fala"
PLATFORMS = ("windows-x86_64", "windows-x86_64-nsis")
VERSION_RE = re.compile(r"\d+\.\d+\.\d+(-rc\.\d+)?")

# --- Ed25519 verification (RFC 8032, section 6) -------------------------------------------

P = 2**255 - 19
Q = 2**252 + 27742317777372353535851937790883648493
D = -121665 * pow(121666, P - 2, P) % P
SQRT_M1 = pow(2, (P - 1) // 4, P)


def _add(a, b):
    x = (a[1] - a[0]) * (b[1] - b[0]) % P
    y = (a[1] + a[0]) * (b[1] + b[0]) % P
    c = 2 * a[3] * b[3] * D % P
    d = 2 * a[2] * b[2] % P
    e, f, g, h = y - x, d - c, d + c, y + x
    return (e * f, g * h, f * g, e * h)


def _mul(s, point):
    acc = (0, 1, 1, 0)
    while s > 0:
        if s & 1:
            acc = _add(acc, point)
        point = _add(point, point)
        s >>= 1
    return acc


def _equal(a, b):
    return (a[0] * b[2] - b[0] * a[2]) % P == 0 and (a[1] * b[2] - b[1] * a[2]) % P == 0


def _recover_x(y, sign):
    if y >= P:
        return None
    x2 = (y * y - 1) * pow(D * y * y + 1, P - 2, P)
    if x2 == 0:
        return None if sign else 0
    x = pow(x2, (P + 3) // 8, P)
    if (x * x - x2) % P != 0:
        x = x * SQRT_M1 % P
    if (x * x - x2) % P != 0:
        return None
    return P - x if (x & 1) != sign else x


def _decompress(raw):
    y = int.from_bytes(raw, "little")
    sign = y >> 255
    y &= (1 << 255) - 1
    x = _recover_x(y, sign)
    return None if x is None else (x, y, 1, x * y % P)


G_Y = 4 * pow(5, P - 2, P) % P
G = (_recover_x(G_Y, 0), G_Y, 1, _recover_x(G_Y, 0) * G_Y % P)


def _sha512_modq(data):
    return int.from_bytes(hashlib.sha512(data).digest(), "little") % Q


def ed25519_verify(public, message, signature):
    a = _decompress(public)
    r = _decompress(signature[:32])
    s = int.from_bytes(signature[32:], "little")
    if a is None or r is None or s >= Q:
        return False
    h = _sha512_modq(signature[:32] + public + message)
    return _equal(_mul(s, G), _add(r, _mul(h, a)))


# --- minisign ------------------------------------------------------------------------------


class ManifestError(Exception):
    pass


def _b64_lines(text, what):
    try:
        return base64.b64decode(text.strip(), validate=True).decode("utf-8").splitlines()
    except ValueError as e:
        raise ManifestError(f"{what} is not base64 of a minisign file: {e}") from e


def parse_pubkey(pubkey):
    """(key id, 32-byte key) from the base64 string Tauri keeps in plugins.updater.pubkey."""
    lines = _b64_lines(pubkey, "plugins.updater.pubkey")
    raw = base64.b64decode(lines[1]) if len(lines) >= 2 else b""
    if len(raw) != 42 or raw[:2] != b"Ed":
        raise ManifestError("plugins.updater.pubkey is not a minisign public key")
    return raw[2:10], raw[10:]


def verify_signature(data, sig_text, pubkey):
    """Checks a Tauri .sig (base64 of a minisign signature file) over `data`; returns the
    trusted comment."""
    key_id, key = parse_pubkey(pubkey)
    lines = _b64_lines(sig_text, "the .sig file")
    if len(lines) < 4 or not lines[2].startswith("trusted comment: "):
        raise ManifestError("the .sig file is not a minisign signature")
    raw = base64.b64decode(lines[1])
    if len(raw) != 74 or raw[:2] not in (b"Ed", b"ED"):
        raise ManifestError("the .sig file has an unknown signature algorithm")
    if raw[2:10] != key_id:
        raise ManifestError(
            f"the .sig was made with key {raw[2:10][::-1].hex().upper()}, "
            f"plugins.updater.pubkey is {key_id[::-1].hex().upper()}"
        )
    message = hashlib.blake2b(data).digest() if raw[:2] == b"ED" else data
    if not ed25519_verify(key, message, raw[10:]):
        raise ManifestError("the signature does not verify against plugins.updater.pubkey")
    comment = lines[2][len("trusted comment: ") :]
    if not ed25519_verify(key, raw[10:] + comment.encode("utf-8"), base64.b64decode(lines[3])):
        raise ManifestError("the trusted comment of the .sig does not verify")
    return comment


# --- manifest ------------------------------------------------------------------------------


def build_manifest(version, exe, notes, pubkey, pub_date, repo=REPO):
    if not VERSION_RE.fullmatch(version):
        raise ManifestError(f"version {version!r} is not X.Y.Z or X.Y.Z-rc.N")
    name = f"Fala_{version}_x64-setup.exe"
    if exe.name != name:
        raise ManifestError(f"the installer is {exe.name!r}; version {version} needs {name!r}")
    if not exe.is_file():
        raise ManifestError(f"{exe} does not exist")
    sig = exe.with_name(exe.name + ".sig")
    if not sig.is_file():
        raise ManifestError(f"{sig.name} is missing: build with TAURI_SIGNING_PRIVATE_KEY set")
    if not notes.strip():
        raise ManifestError("the notes are empty: use the CHANGELOG.md entry of this version")
    sig_text = sig.read_text(encoding="utf-8").strip()
    comment = verify_signature(exe.read_bytes(), sig_text, pubkey)
    signed_file = re.search(r"\bfile:([^\t]+)", comment)
    if signed_file and signed_file.group(1) != name:
        raise ManifestError(f"the .sig signs {signed_file.group(1)!r}, not {name!r}")
    entry = {
        "signature": sig_text,
        "url": f"https://github.com/{repo}/releases/download/v{version}/{name}",
    }
    return {
        "version": version,
        "notes": notes.strip(),
        "pub_date": pub_date,
        "platforms": {platform: dict(entry) for platform in PLATFORMS},
    }


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--version", required=True)
    parser.add_argument("--exe", required=True, type=Path)
    parser.add_argument("--notes-file", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--conf", type=Path, default=CONF)
    parser.add_argument("--pub-date", help="RFC 3339; defaults to now (UTC)")
    args = parser.parse_args(argv)
    pub_date = args.pub_date or datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    try:
        pubkey = json.loads(args.conf.read_text(encoding="utf-8"))["plugins"]["updater"]["pubkey"]
        manifest = build_manifest(
            args.version, args.exe, args.notes_file.read_text(encoding="utf-8"), pubkey, pub_date
        )
    except ManifestError as e:
        print(f"error: {e}", file=sys.stderr)
        return 1
    args.output.write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"{args.output}: {args.version}, signature verified, {len(PLATFORMS)} platform keys")
    return 0


if __name__ == "__main__":
    sys.exit(main())
