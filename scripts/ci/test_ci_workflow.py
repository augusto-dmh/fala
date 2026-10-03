#!/usr/bin/env python3
"""Structural checks on .github/workflows/ci.yml and .github/gitleaks.toml.

Run from the repo root: python3 scripts/ci/test_ci_workflow.py [-k <test name>]
Needs PyYAML (preinstalled on GitHub runners; `pip install pyyaml` elsewhere).
"""

import re
import subprocess
import tomllib
import unittest
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / ".github" / "workflows" / "ci.yml"
GITLEAKS_TOML = ROOT / ".github" / "gitleaks.toml"


def workflow() -> dict:
    return yaml.safe_load(WORKFLOW.read_text())


def triggers(wf: dict) -> dict:
    # YAML 1.1 reads the bare key `on` as the boolean True.
    return wf.get("on", wf.get(True))


def run_steps(job: dict) -> list[str]:
    return [s["run"] for s in job["steps"] if "run" in s]


class CiWorkflow(unittest.TestCase):
    def test_windows_runs_on_every_trigger(self):
        wf = workflow()
        job = wf["jobs"]["windows"]
        self.assertEqual(job["runs-on"], "windows-latest")
        self.assertNotIn("if", job)
        on = triggers(wf)
        self.assertIn("pull_request", on)
        self.assertEqual(on["push"]["branches"], ["main"])
        self.assertIn("workflow_dispatch", on)

    def test_windows_runs_plain_cargo_test_after_clippy(self):
        runs = [r.strip() for r in run_steps(workflow()["jobs"]["windows"])]
        self.assertIn("cargo test", runs)
        self.assertEqual(runs.count("cargo test"), 1)
        clippy = [i for i, r in enumerate(runs) if r.startswith("cargo clippy")]
        self.assertTrue(clippy, "no clippy step")
        self.assertLess(clippy[0], runs.index("cargo test"))
        for r in runs:
            if r.startswith("cargo test"):
                for flag in ("-p ", "--package", "--workspace", "--exclude", "--skip"):
                    self.assertNotIn(flag, r)

    def test_windows_failure_is_not_masked(self):
        job = workflow()["jobs"]["windows"]
        self.assertNotIn("continue-on-error", job)
        for step in job["steps"]:
            self.assertNotIn("continue-on-error", step, step)

    def test_secrets_job_shape(self):
        wf = workflow()
        job = wf["jobs"]["secrets"]
        self.assertEqual(job["runs-on"], "ubuntu-24.04")
        self.assertNotIn("if", job)
        self.assertEqual(job["permissions"], {"contents": "read"})
        on = triggers(wf)
        self.assertIn("pull_request", on)
        self.assertEqual(on["push"]["branches"], ["main"])
        checkout = job["steps"][0]
        self.assertEqual(checkout["uses"], "actions/checkout@v5")
        self.assertEqual(checkout["with"]["fetch-depth"], 0)

    def test_secrets_selftest_runs_before_scan(self):
        steps = workflow()["jobs"]["secrets"]["steps"]
        selftest = [i for i, s in enumerate(steps) if s.get("run", "").strip() == "scripts/ci/test-check-secrets.sh"]
        scan = [i for i, s in enumerate(steps) if s.get("run", "").strip() == "scripts/check-secrets.sh"]
        self.assertEqual(len(selftest), 1)
        self.assertEqual(len(scan), 1)
        self.assertLess(selftest[0], scan[0])
        for i in (selftest[0], scan[0]):
            self.assertNotIn("continue-on-error", steps[i])

    def test_only_github_token_secret(self):
        # Only expressions count: script names like check-secrets.sh are not references.
        exprs = re.findall(r"\$\{\{(.*?)\}\}", WORKFLOW.read_text(), re.DOTALL)
        found = {m for e in exprs for m in re.findall(r"\bsecrets\.([A-Za-z0-9_]+)", e)}
        self.assertEqual(found, {"GITHUB_TOKEN"})


class GitleaksConfig(unittest.TestCase):
    def test_default_rules_and_no_allowlist(self):
        cfg = tomllib.loads(GITLEAKS_TOML.read_text())
        self.assertIs(cfg["extend"]["useDefault"], True)
        self.assertEqual(set(cfg), {"extend"}, "only [extend]: no allowlist, no custom rules")


class Boundaries(unittest.TestCase):
    ALLOWED = (".github/", "scripts/", ".specs/features/ci-windows-secrets/")

    def test_diff_stays_inside_track(self):
        out = subprocess.run(
            ["git", "diff", "--name-only", "origin/main...HEAD"],
            cwd=ROOT, check=True, capture_output=True, text=True,
        ).stdout.split()
        self.assertTrue(out, "empty diff against origin/main")
        outside = [f for f in out if not f.startswith(self.ALLOWED)]
        self.assertEqual(outside, [])


if __name__ == "__main__":
    unittest.main()
