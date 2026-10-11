# updater-on verification

Machine: Windows 11 (Alienware), `CARGO_TARGET_DIR='C:\f\updater'`. Date: 2026-10-11.
Verdict: **FAIL until the maintainer's steps below are done.** The code checks pass. C12 fails only on the placeholder key, and the end-to-end update is unproven (L-004).

| Check | Result | Evidence |
| --- | --- | --- |
| C1 gate | PASS | `cargo test -p fala --lib`: `app_update::tests::refuses_during_each_capture ... ok` |
| C2 second key | PASS | `second_key_only_after_a_signature_failure ... ok` |
| C3 no downgrade | PASS | `only_a_newer_version_is_offered ... ok` (versions parsed by the plugin's `RemoteRelease`) |
| C4 "Depois" | PASS | `later_dismisses_that_version_only ... ok` |
| C5 interval | PASS | `checks_every_six_hours_with_jitter ... ok` |
| C6 NSIS | PASS | `installer.nsi:852-857`: `RunAsUser` only inside `${If} ${Errors}` after `GetOptions "/NORELAUNCH"`. The installer itself was not built or run here |
| C7 Rust gates | PASS | `cargo clippy -p fala --all-targets -- -D warnings` exit 0. `cargo test -p fala --lib`: 496 passed, 0 failed, 1 ignored (the bindings export). The workspace-wide runs are the pre-push hook of each branch |
| C8 manifest | PASS | `python scripts/ci/test_gen_latest_json.py`: 12 tests OK, including the RFC 8032 vector and `test_builds_both_platform_keys` |
| C9 refusals | PASS | same run: missing `.sig`, other version, bad version, other key, tampered installer, `.sig` of another file, empty notes, placeholder key, CLI writes nothing on refusal |
| C10 key script | PASS (static) | `test_key_script_writes_only_the_public_key` OK. The pubkey rewrite was dry-run in PowerShell on a copy of `tauri.conf.json` with a fake key: 1 line changed, valid JSON. The script itself was **not run**, because generating the key is the maintainer's job |
| C11 RELEASE.md | PASS | `python -X utf8 scripts/ci/test_release_config.py`: the section tests pass; `git diff feat/updater-on -- docs/RELEASE.md` shows no line from "Versão e changelog" |
| C12 config guard | **FAIL (expected)** | 18 tests, 1 failure: `test_updater_on` refuses the placeholder `pubkey`. `test_guard_accepts_a_real_key_and_rejects_the_rest` passes, so a real key turns it green |
| C13 shell | PASS | `bun src/components/shell.test.tsx` exit 0 (C28 to C32 ok) |
| C14 frontend can't install | PASS | `! grep -rn plugin-updater src && ! grep -rn '"updater:' apps/desktop/capabilities` |
| C15 What's New | PASS | `bun src/components/whats-new/pickReleaseNote.test.ts`: ok |
| C16 strings/lint | PASS | `bun run check:translations`, `bun run lint`, `bun run format:check`, `bunx tsc --noEmit` all exit 0; no top-level `footer` key left |

## Unproven (outside the table, L-004)

- End to end on Windows: install a released version with the updater, publish the next one as a prerelease, point `FALA_UPDATE_MANIFEST_URL` at it, see the offer, get the refusal during a dictation, restart to update, then check settings, history and What's New; repeat with "Sair" (the app must stay closed). This needs the maintainer's key and two signed releases. The steps are in `docs/RELEASE.md` › "Validar no Windows".
- The tray item, the About row and the toast were not seen running: no dev app was started here.
