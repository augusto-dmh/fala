#!/usr/bin/env python3
"""Structural checks on .github/workflows/ci.yml.

Run from the repo root: python3 scripts/ci/test_ci_workflow.py [-k <test name>]
Needs PyYAML (preinstalled on GitHub runners; `pip install pyyaml` elsewhere).
"""

import unittest
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / ".github" / "workflows" / "ci.yml"


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





if __name__ == "__main__":
    unittest.main()
