# language-picker - verification

**Verdict**: PASS
**Profile**: light
**Diff range**: 481633b..fa615bf (3 commits: 8ba44a3, 0c35140, fa615bf)
**Round**: 1 - full
**Verifier**: independent sub-agent (author != verifier). Author: the desktop-phase1b executor sub-agent. Verifier: a fresh sub-agent with no inherited build context, dispatched by the orchestrator.

All proofs ran at `fa615bf` in a scratch worktree (`/tmp/claude-1000/verify-lp`, detached, removed
afterwards). Cargo used `CARGO_TARGET_DIR=/home/augusto/projects/fala/target CARGO_BUILD_JOBS=2`, one
command at a time. The real worktree was read only; `git status --porcelain` there was empty before
the run, and this file is the only thing written to it.

## Checks

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | `tray_language_choice`: `pt`,`pt-BR` -> `Some("pt-BR")`; `en`,`en-US` -> `Some("en")`; `auto`,`es`,`""` -> `None` (7 entries) | `cargo test -p fala --lib -- tray::tests::tray_language_choice_maps_the_stored_intent ...` (batched, see Gate) - `test tray::tests::tray_language_choice_maps_the_stored_intent ... ok` | `apps/desktop/src/tray.rs:812` - `assert_eq!(tray_language_choice(stored), expected, "stored {stored:?}")` over the 7-row table at `tray.rs:802-810` | PASS |
| C2 | `MenuInputs` differing only in `language_choice` compare unequal (`pt-BR` vs `en`, `en` vs `None`) | same batch - `test tray::tests::language_choice_change_rebuilds_the_menu ... ok` | `apps/desktop/src/tray.rs:827` - `assert_ne!(pt, en)`; `tray.rs:828` - `assert_ne!(en, none)`; field wired from the store at `tray.rs:365` | PASS |
| C3 | ids `dictation_language:pt-BR`/`:en`; parse accepts those two, rejects `:es`, `:`, `:PT-BR`, `model_select:pt-BR` | same batch - `test tray::tests::language_item_ids_round_trip_only_for_known_tags ... ok` | `apps/desktop/src/tray.rs:833` - `assert_eq!(language_item_id("pt-BR"), "dictation_language:pt-BR")`; `tray.rs:839` - `assert_eq!(parse_language_item("dictation_language:en"), Some("en"))`; `tray.rs:846` - `assert_eq!(parse_language_item(id), None, "id {id:?}")` over the 4 rejected ids at `tray.rs:841-844` | PASS |
| C4 | language submenu only in the idle branch of `build_menu`, one `CheckMenuItem` per tag checked by `inputs.language_choice` | 3 structural proofs, each exit 0 (busy-branch awk piped to negated grep, idle-branch awk piped to grep, `grep -q 'Some(\*tag) == inputs.language_choice'`) | `apps/desktop/src/tray.rs:544` busy branch (`let menu = if inputs.busy {`, ends `:561`) has no `language_submenu`; `tray.rs:615` - `&language_submenu,` in the idle `Menu::with_items`; `tray.rs:593` - `Some(*tag) == inputs.language_choice,` | PASS |
| C5 | tray handler passes the `parse_language_item` tag, and only it, to `change_selected_language_setting` | `grep -A4 'parse_language_item(id)' apps/desktop/src/lib.rs` piped to `grep -q change_selected_language_setting`, exit 0 | `apps/desktop/src/lib.rs:326` - `if let Some(tag) = tray::parse_language_item(id) {`; `lib.rs:328` - `shortcut::change_selected_language_setting(app.clone(), tag.to_string())`; an unknown `dictation_language:*` id hits the arm at `lib.rs:325` and is dropped (no fall-through) | PASS |
| C6 | `change_selected_language_setting` emits `settings-changed` with `"setting": "selected_language"` and calls `tray::update_tray_menu` | 2 structural proofs (awk over the fn body), each exit 0 | `apps/desktop/src/shortcut/mod.rs:650` - `"setting": "selected_language",` inside `app.emit("settings-changed", ...)`; `shortcut/mod.rs:654` - `crate::tray::update_tray_menu(&app);` | PASS |
| C7 | `tray.dictationLanguage`/`languagePtBr`/`languageEn` = "Idioma do ditado"/"Português (Brasil)"/"Inglês" (pt) and "Dictation language"/"Portuguese (Brazil)"/"English" (en); key parity; `build.rs` generates the fields | python3 literal-compare exit 0; `bun run check:translations` exit 0 ("PT: All keys present", 446 keys); cargo batch compiled and printed `Generated tray translations: 2 languages, 11 fields` | `src/i18n/locales/pt/translation.json:8` - `"dictationLanguage": "Idioma do ditado"` (`:9`, `:10` the other two); `src/i18n/locales/en/translation.json:8` - `"dictationLanguage": "Dictation language"` (`:9`, `:10`); generated fields consumed at `apps/desktop/src/tray.rs:581` (`&strings.dictation_language`), `:584` (`language_en`), `:586` (`language_pt_br`) | PASS |
| C8 | `get_default_settings().selected_language` and `from_value::<AppSettings>(json!({}))` are `"pt-BR"` | same batch - `test settings::tests::default_selected_language_is_pt_br ... ok` | `apps/desktop/src/settings.rs:1803` - `assert_eq!(get_default_settings().selected_language, "pt-BR")`; `settings.rs:1806` - `assert_eq!(from_empty.selected_language, "pt-BR")`; both assemblies read directly: `settings.rs:567` (`"pt-BR".to_string()` in `default_selected_language`, used by `#[serde(default = ...)]` at `:424`) and `settings.rs:938` (`selected_language: default_selected_language()`) | PASS |
| C9 | schema-2 store with `"auto"`/`"pt"`/`"es"` keeps the value after `apply_settings_migrations` | same batch - `test settings::tests::stored_selected_language_loads_unchanged ... ok` | `apps/desktop/src/settings.rs:1823` - `assert_eq!(settings.selected_language, stored, "stored '{stored}'")` over `["auto", "pt", "es"]` (`:1811`), after strict `from_value` (`:1820`) and `apply_settings_migrations` (`:1822`) | PASS |
| C10 | `effective_language("pt-BR", ["en","pt"], true)` and `(..., false)` both == `"pt"` | same batch - `test managers::model::tests::test_effective_language_resolves_pt_br_intent_to_model_pt ... ok` | `apps/desktop/src/managers/model.rs:2722` - `assert_eq!(effective_language("pt-BR", &languages, true), "pt")`; `model.rs:2723` - same with `false` | PASS |

### Named tests exist, are new, and ran

All six named tests are added in this diff (`git diff 481633b..fa615bf`). None resolves to a test
the feature did not touch. Located by `grep -n`: `tray.rs:801`, `:817`, `:832`;
`settings.rs:1802`, `:1810`; `model.rs:2718`. All six appear by name in the cargo output as `... ok`.
`running 6 tests` / `6 passed; 0 failed; 294 filtered out` rules out a filter that matched nothing.

### Structural proofs can fail

Every grep proof was run against a deliberate mutant in the scratch worktree and went red. Each
mutant was reverted with `git checkout`, and the scratch porcelain was empty afterwards:

| Proof | Mutant applied | Exit under mutant |
| --- | --- | --- |
| C4 busy branch (negated) | `&language_submenu,` inserted into the busy `Menu::with_items` | 1 |
| C4 idle branch | `&language_submenu,` removed from the idle `Menu::with_items` | 1 |
| C4 checked state | `Some(*tag) == inputs.language_choice` replaced by `false` | 1 |
| C5 | handler call replaced by `Ok::<(), String>(())` | 1 |
| C6 setting field | `"setting": "selected_language",` deleted from the fn | 1 |
| C6 tray update | `crate::tray::update_tray_menu(&app);` deleted from the fn | 1 |
| C7 literals | pt `languageEn` changed to `"English"` | 1 |
| C7 parity | pt `dictationLanguage` key deleted (`bun run check:translations`) | 1 |

(This is a light-profile can-fail check on the grep proofs only. It is not the standard-profile
fault injection over the test assertions.)

## Level and sampling

- **C4, C5 and C6 are structural.** They prove the wiring is present in the source, not that it runs.
  `checks.md` says why: `build_menu` and the menu handler need an `AppHandle`. The parts that make
  decisions (the choice and the id parsing) are covered behaviourally by C1 and C3. I accept that at
  `light`. A runtime proof of the click path would need the Windows session.
- **Precision note (C4, idle-branch proof).** `awk '/} else {/,/^    };/'` matches every
  `} else {` ... `    };` range in `tray.rs`, not only the one in `build_menu`. Today
  `&language_submenu` appears only once in the file (`tray.rs:615`), so the proof holds. A future
  `&language_submenu` in another else-block would satisfy it falsely. This is not a failure.
- **C9 samples schema version 2 only.** I read `apply_settings_migrations` (`settings.rs:1107-1170`)
  and checked the `< 1` and `< 2` paths. Neither touches `selected_language`, so v0/v1 stores keep
  their value too. The stored-language default therefore does not break loading existing stores: a
  present key wins over `#[serde(default)]`, and no migration rewrites it.
- **Swept `idempotency` cites C2**, but C2 only asserts inequality. Equality for the same choice comes
  from the derived `PartialEq` on `MenuInputs`, which is trivially true. That is a minor imprecision
  in the sweep, not a gap in behaviour.

## Swept existing (re-read)

| Row | Cited constraint | Found |
| --- | --- | --- |
| failure modes | `write_settings` herdado; menu reconstruído do settings relido | yes - `settings.rs:1216` `pub fn write_settings(app, settings)` returns `()`; `tray.rs:258` `update_tray_menu` -> `sync_tray`, and `compute_desired` rereads `settings::get_settings(app)` (`tray.rs:341`) |
| concurrency | snapshots com sequência e coalescência (cabeçalho do módulo `tray.rs`) | yes - `tray.rs:3-9` module doc: single desired-state snapshot, single applier on the main thread, bursts coalesced |

## AGENTS.md rules for `apps/desktop`

- **No gratuitous reformatting.** The diff removes 4 lines, and each is a needed edit (the two `"auto"`
  defaults, the move of `language` into a clone, and the widened `use super::{...}` in tests).
  `rustfmt --edition 2021 --check` on the five touched `.rs` files exits 0. `bunx prettier --check`
  on both locale files passes.
- `cargo clippy -p fala --all-targets -- -D warnings`: exit 0.
- `scripts/check-brand.sh`: `ok: no Handy branding outside the allowlist`.
- No `#[cfg(target_os)]` added. No `unwrap`/`expect` added outside tests: the `unwrap()` at
  `lib.rs:335` is in the inherited `model_select:` arm, which this diff does not touch. No new
  dependency. `build.rs` itself is unchanged; it derives the struct fields from the en `tray`
  section and now reports 11 fields.
- `tray_language_choice` reuses the existing `pub(crate) canonical_language_code`
  (`managers/model.rs:103`). The diff does not change its visibility.

## Outside the checks table (not a check row)

`checks.md` declares no Windows-only check ("Nenhum check fica só no Windows"). It sends the visual
check of the tray submenu on Windows 11 to the session's `TODO(windows)` checklist. I did not move or
remove any row to reach this verdict. The Checks table above is exactly C1-C10 from `checks.md`. The
manual Windows step in the plan's S1 independent test is still **not verified**: open the tray, switch
to English, dictate an English sentence. The same goes for the plan's note that an existing store with
`auto` shows the submenu with no item checked. Both need the Windows session.

## Gate

- `cargo test -p fala --lib -- <6 named tests>`: 6 passed, 0 failed (294 filtered out), exit 0
- `cargo test -p fala --lib`: 300 passed, 0 failed
- `cargo clippy -p fala --all-targets -- -D warnings`: exit 0
- `bun run check:translations`: exit 0
- 10/10 checks proven with located `file:line` evidence. Under `light` there is no fault injection
  over the test assertions and no Coverage recompute.

## Lessons

This is a clean PASS with no failed check, unproven member or surviving mutant, so no lesson was
recorded. The C4 awk-range precision note is minor and left to the orchestrator.
