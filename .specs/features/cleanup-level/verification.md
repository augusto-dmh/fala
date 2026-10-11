# cleanup-level - verification

**Verdict**: PASS
**Profile**: light
**Diff range**: `origin/main...HEAD` = cca0a87..d495de4 (2 commits: 1a29a7d desktop, d495de4 ui)
**Round**: 1 - full
**Verifier**: independent sub-agent (author != verifier). It did not write the code and has no build context from the author.

All proofs ran at `d495de4` in the real worktree (`C:\dev\fala\.houston\worktrees\cleanup-level-cards`),
because the cargo target dir is tied to it. Cargo used `CARGO_TARGET_DIR=C:\f\cleanup-level-cards`, one
command at a time under the shared `cargo-lock.txt`. Before the run `git status --porcelain` showed only
`?? .specs/features/cleanup-level/` (the plan and checks, not tracked yet). Every mutant was undone with
`git checkout -- <file>`, and the porcelain after each one was the same single line. This file is the
only thing the Verifier wrote.

## Checks

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | `get_default_settings()`, `from_value(json!({}))` and a migrated schema-2 store without the key all give `CleanupLevel::Light` | `cargo test -p fala --lib -- <4 named tests>` (batched, see Gate) - `test settings::tests::cleanup_level_defaults_to_light ... ok` | `apps/desktop/src/settings.rs:2152` - `assert_eq!(get_default_settings().cleanup_level, CleanupLevel::Light)`; `:2155` - `assert_eq!(from_empty.cleanup_level, ...)`; `:2157` - `assert_eq!(old_store.cleanup_level, ...)` after `migrated(schema_2_store(..))`. Sources: `#[default] Light` at `settings.rs:342`, `#[serde(default)]` on the field at `:673-675`, `cleanup_level: CleanupLevel::default()` at `:1154` | PASS |
| C2 | `"extreme"` and `42` fail the strict parse; `salvage_settings` gives `Light` and keeps `selected_model` and `onboarding_completed` | same batch - `test settings::tests::invalid_cleanup_level_is_salvaged_to_light ... ok` | `settings.rs:2172` - `serde_json::from_value::<AppSettings>(stored.clone()).is_err()`; `:2177` - `assert_eq!(salvaged.cleanup_level, CleanupLevel::Light, "{invalid}")`; `:2178` - `selected_model == "parakeet-tdt-0.6b-v3"`; `:2179` - `assert!(salvaged.onboarding_completed)`; loop over both values at `:2162`. The warning is the inherited one at `settings.rs:1278` (`warn!("Dropping invalid settings field '{key}', keeping its default")`), reached from the load path at `:1206-1210` | PASS |
| C3 | each of the 4 literals loads, serializes back to the same literal, and maps to the `fala_postproc::CleanupLevel` with that `as_str()` | same batch - `test settings::tests::stored_cleanup_levels_round_trip ... ok` | `settings.rs:2194` - `assert_eq!(settings.cleanup_level, level, "stored '{literal}'")`; `:2196` - `to_value(&settings)["cleanup_level"] == literal`; `:2199` - `assert_eq!(fala_postproc::CleanupLevel::from(level).as_str(), literal)`; 4-row table at `:2185-2191`. Mapping: `impl From<CleanupLevel> for fala_postproc::CleanupLevel` at `settings.rs:348` | PASS |
| C4 | with the fake Gemini, for each level the `systemInstruction` holds that level's `instruction()` and none of the other three; the default holds `Light`'s | same batch - `test llm_auto::tests::cleanup_level_reaches_the_llm_prompt ... ok` | `apps/desktop/src/llm_auto.rs:371-372` - `assert_eq!(prompt.contains(other.instruction()), other == level, ..)` for every `other` in `CleanupLevel::ALL`, inside the 4-level loop; `:367` - each run is `llm_produced`; `:380` - default store prompt `contains(CleanupLevel::Light.instruction())`. Production line: `llm_auto.rs:55` - `.with_cleanup_level(settings.cleanup_level.into())` | PASS |
| C5 | `change_cleanup_level_setting` assigns `settings.cleanup_level = level` and calls `write_settings`; registered in `collect_commands!`; `bindings.ts` has the method and the union | 5 structural proofs from `checks.md`, each exit 0 | `apps/desktop/src/shortcut/mod.rs:988` - `pub fn change_cleanup_level_setting(`; `:993` - `settings.cleanup_level = level;`; `:994` - `settings::write_settings(&app, settings);`; `apps/desktop/src/lib.rs:767` - `shortcut::change_cleanup_level_setting,` inside `collect_commands!` of `specta_builder`; `src/bindings.ts:334` - `async changeCleanupLevelSetting(level: CleanupLevel)`; `bindings.ts:1300` - `export type CleanupLevel = "verbatim" \| "light" \| "medium" \| "heavy"`. Consumer: `src/stores/settingsStore.ts:153-154` | PASS |
| C6 | one `radiogroup`, four `radio` in order `verbatim, light, medium, heavy`, each with label, description and example from `settings.llm.cleanup` | `bun src/components/settings/cleanupLevel.test.tsx` - `C6 ok`, exit 0 | `cleanupLevel.test.tsx:58` - one `role="radiogroup"`; `:60-63` - `data-level` order deep-equal; `:65-72` - each card has `role="radio"` and contains `label`, `description`, `example`; `:74-79` - the raw dictation is shown, but not inside the cards; `:81-83` - group `aria-labelledby` points to the title id. Source: `CleanupLevelCards.tsx:57-60` (radiogroup), `:65-76` (radio) | PASS |
| C7 | with each stored value, only its radio has `aria-checked="true"`, `tabindex="0"` and `border-accent`; the others `tabindex="-1"` | same run - `C7 ok` | `cleanupLevel.test.tsx:88-107` - loops all 4 values (wider than the claim, which names only `medium`), asserts `aria-checked`, `tabindex` and `border-accent` per card. Source: `CleanupLevelCards.tsx:72-73`, `:78-80` | PASS |
| C8 | `levelForKey` moves next/previous with wrap for the four arrows, Home/End first/last, `null` for other keys | same run - `C8 ok` | `cleanupLevel.test.tsx:113-130` - 13-row table, both wraps in both directions, `Home`, `End`, and `Enter`/`" "`/`Tab` -> `null`. Source: `cleanupLevelModel.ts:15-34` | PASS |
| C9 | `dimmed` adds `opacity-60` and shows `settings.llm.cleanup.inactive`; neither without it; radios never `disabled` | same run - `C9 ok` | `cleanupLevel.test.tsx:139-140` - group classes with/without `opacity-60`; `:141-142` - inactive line present/absent; `:143-145` - no `disabled` on any radio. Source: `CleanupLevelCards.tsx:52-56`, `:60` | PASS |
| C10 | footer shown; pt footer and inactive copy as defined; `LlmSettings.tsx` mounts the cards right after `ToggleSwitch` with `dimmed={!enabled \|\| !apiKey}` | same run - `C10 ok` | `cleanupLevel.test.tsx:152` - footer in the render; `:153-156` - exact pt footer; `:157-160` - inactive contains `ale quando a IA formata o ditado`; `:161-169` - source regex over `LlmSettings.tsx` and the `onChange` wiring. Source: `LlmSettings.tsx:51-55`; `src/i18n/locales/pt/translation.json:300-301`. The footer names real UI strings: `pt/translation.json:570` (`"undoAiEdit": "Desfazer edição da IA"`) and `:24` (`"home": "Início"`) | PASS |
| C11 | `settings.llm.cleanup` keys equal in pt and en, all non-empty, pt examples as defined; global parity passes | same run - `C11 ok`; `bun run check:translations` exit 0 (`PT: All keys present`, 608 reference keys) | `cleanupLevel.test.tsx:179-182` - sorted key paths deep-equal; `:183-193` - every leaf a non-empty string in both locales; `:195-222` - pt `raw`, the four labels and the four examples by exact string. Source: `pt/translation.json:294-324`, `en/translation.json:294-324` | PASS |

### Named tests exist, are new, and ran

All four Rust tests are added in this diff (`git diff origin/main...HEAD`): `settings.rs:2151`, `:2161`,
`:2184`; `llm_auto.rs:355`. `src/components/settings/cleanupLevel.test.tsx` is a new file in d495de4.
None of the proofs resolves to a test the feature did not touch. The cargo batch printed all four by name as `... ok`; `running 4 tests` / `4 passed; 0 failed; 427 filtered out` rules out a filter that matched nothing.

## Structural proofs can fail

Each grep proof of C5 and the source-regex part of C10 was run against a deliberate mutant and went red.
For the frontend script I also mutated one behaviour per check (cheap: no compile).

| Proof | Mutant applied | Result under mutant |
| --- | --- | --- |
| C5 assignment | `settings.cleanup_level = level;` -> `let _ = level;` | exit 1 |
| C5 write | `settings::write_settings(&app, settings);` deleted from the fn | exit 1 |
| C5 registration | `shortcut::change_cleanup_level_setting,` deleted from `lib.rs` | exit 1 |
| C5 union | `"heavy"` dropped from `export type CleanupLevel` | exit 1 |
| C5 method | `changeCleanupLevelSetting(level: string)` | exit 1 |
| C10 `dimmed` | `dimmed={!enabled \|\| !apiKey}` -> `dimmed={!enabled}` | `C10` AssertionError (regex), exit 1 |
| C10 position | cards block moved below the API key `SettingContainer` | `C10` AssertionError, exit 1 |
| C10 position | cards block deleted from `LlmSettings.tsx` | `C10` AssertionError, exit 1 |
| C7 | `tabIndex={checked ? 0 : -1}` -> `tabIndex={0}` | `C7` AssertionError `verbatim: light tabindex` |
| C8 | forward wrap removed (`Math.min(index + 1, last)`) | `C8` AssertionError `heavy ArrowRight` |
| C9 | `disabled={dimmed}` added to each radio | `C9` AssertionError `verbatim not disabled` |
| C9 | inactive line never rendered (`{false && (`) | `C9` AssertionError `inactive line` |
| C11 | pt `heavy.example` shortened | `C11` AssertionError (strict equal) |
| C4 (behavioural) | `.with_cleanup_level(settings.cleanup_level.into())` removed from `formatter_at` | `cleanup_level_reaches_the_llm_prompt ... FAILED` at `llm_auto.rs:369` ("Verbatim asked, verbatim in the prompt?") |
| C3 + C4 (behavioural) | `CleanupLevel::Medium => Self::Heavy` in the `From` impl | `stored_cleanup_levels_round_trip ... FAILED` at `settings.rs:2199`; `cleanup_level_reaches_the_llm_prompt ... FAILED` at `llm_auto.rs:370` |

Two mutants **survived**, both outside the letter of the checks (see Level and sampling):

| Proof | Mutant applied | Result under mutant |
| --- | --- | --- |
| C10 "right below the toggle" | `<hr />` inserted between `<ToggleSwitch ... />` and `<CleanupLevelCards` | all six lines `ok`, exit 0 |
| C6-C11 (no check covers it) | `onKeyDown={handleKeyDown}` and `onClick={() => onChange(level)}` both deleted from the radios | all six lines `ok`, exit 0 |

## Level and sampling

- **C5 is structural.** The command needs an `AppHandle`; `checks.md` proves it by grep. The body is three
  lines in the same shape as `change_llm_enabled_setting` (`shortcut/mod.rs:979`), read in full. I accept
  that at `light`. The `lib.rs` grep proves the line exists, not that it sits inside `collect_commands!`;
  I read it there (`lib.rs:767`, in `specta_builder`).
- **AC 8 is wider than C8.** The plan says pressing an arrow on a card chooses the next level. C8 proves
  only the pure `levelForKey`. That the radios call it (`onKeyDown={handleKeyDown}`,
  `CleanupLevelCards.tsx:76`), that `onChange` fires, and that focus follows (`:29-33`) is not proven by any
  check, nor is the click (`:75`). The surviving mutant above shows it. A static render cannot fire events;
  a DOM test (happy-dom) or a source grep would close it. Not a failure of C8 as written.
- **C10 "logo depois do `ToggleSwitch`" is approximate.** The lazy `[\s\S]*?\/>\s*<CleanupLevelCards`
  can extend past any self-closing element placed between the two, so an `<hr />` in between survives.
  Moving the cards below a `SettingContainer` (closed by `</SettingContainer>`) is caught. Today the cards
  are directly after the toggle (`LlmSettings.tsx:50-51`), read by eye.
- **C6 claim wording.** `checks.md` says each radio has "rótulo, descrição, o exemplo e o bruto". The test
  asserts the opposite for the raw dictation: it appears once above the grid and *not* inside the cards
  (`cleanupLevel.test.tsx:74-79`). The code matches AC 6 of the plan (each card has description and
  example). The check text should drop "e o bruto" or say "o bruto uma vez, fora dos cards".
- **AC 6 "one-line description"** is not asserted as one line; the descriptions are short sentences.
- **AC 2 warning** is not asserted by C2 (no log capture). `checks.md` declares it as the inherited
  `salvage_settings` warning; I read it at `settings.rs:1278`: it logs `'{key}'` only.
- **Late edit and "Aplicar edição da IA"** (Coverage row "caminhos que pedem ao Gemini") are structural, as
  `checks.md` says. Confirmed: `crates/postproc/src/lib.rs:252` - `let cleanup = self.cleanup;` is moved into
  the thread whose single `gemini.call` result feeds both the in-time text and the `LateEdit`
  (`lib.rs:254-275`). `llm_auto::formatter` is the only `Postprocessor::new` in `apps/desktop`
  (`llm_auto.rs:50`), called per dictation from `actions.rs:548` with freshly read settings. The fuzzy
  refix at `llm_auto.rs:139` still passes `CleanupLevel::default()`, but that path is rules-only (LLM did not
  produce); `Rules` does not read `cleanup`, only `gemini.rs:91` does.
- **Plan vs. code, "carregando" row.** The plan's Observable table says "`isUpdating` desabilita durante a
  gravação". The code does not pass `isUpdating` to the cards, on purpose: `checks.md` Handoff records it as
  settled mid-build (focus would be lost during arrow navigation). The plan row is stale.

## AGENTS.md rules

- `rustfmt --edition 2021 --check` on the four touched `.rs` files: exit 0. `bunx prettier --check` on the
  seven touched frontend files: all formatted. `bunx eslint` on the five touched TS/TSX files: exit 0 (no JSX
  literal). `bunx tsc --noEmit`: exit 0.
- `scripts/check-brand.sh`: `ok: no Handy branding outside the allowlist`.
- No `unwrap`/`expect` added outside tests. No `#[cfg(target_os)]`. No new dependency; `fala-postproc`
  gains no `specta` (the desktop keeps its own enum, `settings.rs:337-357`).
- `src/bindings.ts`: the diff adds only the three hunks of this feature (method, field, type), as the
  Handoff says.
- No dictated content logged; no API key touched.

## Outside the checks table (not a check row)

The manual Windows check in `checks.md` (switch to "Forte", dictate more than 15 words in Notepad with a
Gemini key, see a shorter text; switch back to "Leve") was **not run**. It needs the user's key and a real
Gemini call. It does not change the verdict, as `checks.md` declares. The effect of each level on a real
dictation, and the runtime late edit per level, therefore stay unverified.

## Gate

- `cargo test -p fala --lib -- <4 named tests>`: 4 passed, 0 failed (427 filtered out), exit 0
- `cargo test -p fala --lib` (after every mutant was reverted): 430 passed, 0 failed, 1 ignored
- `cargo clippy -p fala --all-targets -- -D warnings`: exit 0
- `bun src/components/settings/cleanupLevel.test.tsx`: `C6 ok` .. `C11 ok`, exit 0
- `bun run check:translations`: exit 0
- 11/11 checks proven with located `file:line` evidence. Under `light` there is no full fault injection
  over the test assertions and no Coverage recompute; the behavioural mutants above are a sample.

## Lessons

PASS with no failed check. Two mutants survived, both outside the literal claims: the keyboard and click
wiring of the radios (AC 8 is wider than C8) and the C10 "right below the toggle" regex, which tolerates
a self-closing element in between. Suggested for the author, not blocking: assert in the test that each
radio has `onKeyDown={handleKeyDown}` and `onClick` (or a DOM test that fires the keys); tighten the C10
regex so nothing sits between the toggle's `/>` and `<CleanupLevelCards`; fix the C6 wording ("e o
bruto") and the stale `isUpdating` row in the plan's Observable table.

## Author follow-up (after round 1)

The four suggestions above were applied in a later commit, `test(ui): ...`; the verifier did not
re-run:

- C8 now also asserts, in the source of `CleanupLevelCards.tsx`, `onClick={() => onChange(level)}`,
  `onKeyDown={handleKeyDown}` and that the handler calls `onChange(next)` and then focuses the next card.
  Mutants with either attribute removed exit 1.
- The C10 regex is `<ToggleSwitch\b[^<]*?\/>\s*<CleanupLevelCards\b`; the `<hr />` mutant now exits 1.
- `checks.md` C6/C8/C10 wording and the plan's Observable "carregando" row were corrected.
