# spike-windows-prep - checks

Profile: light
Plan: `.specs/features/spike-windows-prep/plan.md`

## Intent

22 checks in 5 slices · 2 one-way doors · 0 open

The three scripts only run on Windows and `pwsh` is not installed here (plan assumption), so
every proof in this change is static: it reads the script or report and asserts the literal the
check names. What a script does at run time is proven on the Windows machine, by running it; each
report's `## Evidência medida` keeps that run as `TODO(windows)`. Proofs run from the repo root.

## Checks

### S1 - contagem do hook · 1 file · 8 KB · ~2k

**C1** - `02-hook-count.ps1` declares `-Mode` (`auto`|`manual`, default `auto`), `-Presses 300`, `-IntervalMs 2000`, `-HoldMs 50`, `-Minutes 10`, `-ExpectedPresses`, `-LogDir` defaulting to `$env:LOCALAPPDATA\br.com.augusto.fala\logs`, `-WindowTitle 'Fala'` (plan AC 1, AC 2)
Proof: `f=scripts/spikes/02-hook-count.ps1; for p in "[ValidateSet('auto', 'manual')][string]\$Mode = 'auto'" '[int]$Presses = 300' '[int]$IntervalMs = 2000' '[int]$HoldMs = 50' '[int]$Minutes = 10' '[int]$ExpectedPresses' '$env:LOCALAPPDATA\br.com.augusto.fala\logs' "[string]\$WindowTitle = 'Fala'"; do grep -qF -- "$p" $f || { echo "missing: $p"; exit 1; }; done`

**C2** - In auto mode the script activates the window, injects F9 (`0x78`) down/up through `SendInput`, holds `$HoldMs` between them and waits `$IntervalMs` between pairs (plan AC 1)
Proof: `f=scripts/spikes/02-hook-count.ps1; for p in 'SetForegroundWindow' 'SendInput' '0x78' 'Start-Sleep -Milliseconds $HoldMs' '$IntervalMs'; do grep -qF -- "$p" $f || { echo "missing: $p"; exit 1; }; done`

**C3** - The script counts lines of `fala.log` written after it starts that match `handy-keys event: binding=transcribe` with `state=Pressed` and, separately, `state=Released`, reading the file as it grows so a log rotation mid-run does not lose lines (plan AC 1, door 2)
Proof: `f=scripts/spikes/02-hook-count.ps1; for p in "'fala.log'" 'handy-keys event: binding=transcribe' 'state=Pressed' 'state=Released' 'FileShare]::ReadWrite' '.Length -lt $script:offset'; do grep -qF -- "$p" $f || { echo "missing: $p"; exit 1; }; done`

**C4** - Manual mode writes the typing instruction to stderr, waits `$Minutes`, and compares with `$ExpectedPresses` (plan AC 2)
Proof: `f=scripts/spikes/02-hook-count.ps1; for p in 'toque F9 a cada ~10 s' 'AddMinutes($Minutes)' '$ExpectedPresses'; do grep -qF -- "$p" $f || { echo "missing: $p"; exit 1; }; done`

**C5** - stdout is one Markdown table with the columns `mode · presses_sent · pressed_logged · released_logged · first_gap_at · foreground_at_end` (plan AC 3)
Proof: `grep -qF '| mode | presses_sent | pressed_logged | released_logged | first_gap_at | foreground_at_end |' scripts/spikes/02-hook-count.ps1`

**C6** - A count different from `presses_sent` exits 1; a missing `fala.log` or window exits 2, and both exit-2 checks come before the first `SendInput` in the file (plan AC 4, AC 5)
Proof: `f=scripts/spikes/02-hook-count.ps1; grep -qF 'exit 1' $f && a=$(grep -nF 'Fail2 "' $f | head -2 | tail -1 | cut -d: -f1) && b=$(grep -nF '[W]::Key(' $f | head -1 | cut -d: -f1) && test "$(grep -cF 'Fail2 "' $f)" -ge 2 && test "$a" -lt "$b"`

### S2 - overlay sem foco · 1 file · 7 KB · ~2k

**C7** - `03-overlay-focus.ps1` declares `-Cycles 200`, `-FalaExe 'fala.exe'`, `-SettleMs 500`, `-PollMs 10` (plan AC 6)
Proof: `f=scripts/spikes/03-overlay-focus.ps1; for p in '[int]$Cycles = 200' "[string]\$FalaExe = 'fala.exe'" '[int]$SettleMs = 500' '[int]$PollMs = 10'; do grep -qF -- "$p" $f || { echo "missing: $p"; exit 1; }; done`

**C8** - Per cycle the script saves the foreground HWND, runs `--toggle-transcription`, polls `FindWindowW` for `Recording` until `IsWindowVisible`, reads `WS_EX_TOPMOST` (`0x8`) from `GWL_EXSTYLE` (`-20`), compares the foreground HWND, runs `--cancel` and waits for the window to hide (plan AC 6)
Proof: `f=scripts/spikes/03-overlay-focus.ps1; for p in 'GetForegroundWindow' '--toggle-transcription' "FindWindowW([NullString]::Value, 'Recording')" 'IsWindowVisible' '0x8' '-20' '--cancel'; do grep -qF -- "$p" $f || { echo "missing: $p"; exit 1; }; done`

**C9** - stdout is one Markdown table with the columns `cycles · overlay_visible · topmost · focus_kept · hidden_after_cancel · show_latency_ms_p50 · show_latency_ms_max`; any count below `cycles` exits 1 (plan AC 7, AC 8)
Proof: `f=scripts/spikes/03-overlay-focus.ps1; grep -qF '| cycles | overlay_visible | topmost | focus_kept | hidden_after_cancel | show_latency_ms_p50 | show_latency_ms_max |' $f && grep -qF 'exit 1' $f`

**C10** - `fala.exe` not runnable, or `Recording` never visible in the first 3 cycles, exits 2 naming the cause (plan AC 9)
Proof: `f=scripts/spikes/03-overlay-focus.ps1; for p in 'Get-Command $FalaExe' '$i -eq 3' 'nunca ficou visível'; do grep -qF -- "$p" $f || { echo "missing: $p"; exit 1; }; done; test "$(grep -cF 'Fail2 "' $f)" -ge 2`

### S3 - colagem com restore · 1 file · 8 KB · ~2k

**C11** - `03-paste-check.ps1` declares `-Apps` defaulting to `chrome, Code, Teams, WindowsTerminal, notepad`, `-Repeats 3`, `-Kinds text,image`, `-FalaExe`, `-Phrase 'teste um dois três'`, `-SpeakSeconds 4`, `-PasteWaitMs 3000` (plan AC 10)
Proof: `f=scripts/spikes/03-paste-check.ps1; for p in "@('chrome', 'Code', 'Teams', 'WindowsTerminal', 'notepad')" '[int]$Repeats = 3' "@('text', 'image')" '[string]$FalaExe' "'teste um dois três'" '[int]$SpeakSeconds = 4' '[int]$PasteWaitMs = 3000'; do grep -qF -- "$p" $f || { echo "missing: $p"; exit 1; }; done`

**C12** - Per combination the script puts `FALA-SPIKE-<guid>` (text) or a generated 8×8 bitmap alone (image) on the clipboard, counts down 5 s on stderr, toggles transcription twice around `$SpeakSeconds`, waits `$PasteWaitMs`, compares the clipboard (text equal; image same size and bytes) and asks `y`/`n` on stderr (plan AC 10)
Proof: `f=scripts/spikes/03-paste-check.ps1; for p in 'FALA-SPIKE-' 'NewGuid()' 'New-Object System.Drawing.Bitmap 8, 8' 'foque' 'Start-Sleep -Seconds $SpeakSeconds' 'Start-Sleep -Milliseconds $PasteWaitMs' 'LockBits' "Read-Host"; do grep -qF -- "$p" $f || { echo "missing: $p"; exit 1; }; done`

**C13** - stdout is one Markdown table with a row per `app · kind · repeat · restored · pasted` and a `total` row; a `n` anywhere exits 1; `fala.exe` not runnable or clipboard unreadable exits 2 before the first combination (plan AC 11-13)
Proof: `f=scripts/spikes/03-paste-check.ps1; grep -qF '| app | kind | repeat | restored | pasted |' $f && grep -qF '| total |' $f && grep -qF 'exit 1' $f && a=$(grep -nF 'Fail2 "' $f | tail -1 | cut -d: -f1) && b=$(grep -nF 'foreach ($app in $Apps)' $f | cut -d: -f1) && test "$a" -lt "$b"`

### S4 - relatórios · 2 files · 12 KB · ~3k

**C14** - Both reports have `## Objetivo`, `## Como reproduzir`, `## Evidência medida`, and no heading containing "Recomenda" (plan AC 14)
Proof: `for f in docs/spikes/02-hook-webview2.md docs/spikes/03-overlay-colagem.md; do test "$(grep -cE '^## (Objetivo|Como reproduzir|Evidência medida)$' $f)" -eq 3 && ! grep -qiE '^#+ .*recomenda' $f || exit 1; done`

**C15** - Each `## Como reproduzir` names `docs/dev/build-windows.md`, `--debug`, `F9` in push-to-talk, `paste_method` = `CtrlV`, `clipboard_handling` = `DontModify`, `reliable_paste` = `false`, and its scripts' invocations with the values of AC 1, 6 and 10 (plan AC 15)
Proof: `for f in docs/spikes/02-hook-webview2.md docs/spikes/03-overlay-colagem.md; do s=$(awk '/^## Como reproduzir/,/^## Evid/' $f); for p in 'docs/dev/build-windows.md' '--debug' 'F9' 'CtrlV' 'DontModify' 'reliable_paste'; do grep -qF -- "$p" <<<"$s" || { echo "$f missing $p"; exit 1; }; done; done; grep -qF -- '-Presses 300 -IntervalMs 2000 -HoldMs 50' docs/spikes/02-hook-webview2.md && grep -qF -- '-Cycles 200' docs/spikes/03-overlay-colagem.md && grep -qF '03-paste-check.ps1' docs/spikes/03-overlay-colagem.md`

**C16** - Each `## Evidência medida` has `TODO(windows)` and the empty table with the exact columns its script prints (plan AC 16)
Proof: `s=$(awk '/^## Evidência medida/,0' docs/spikes/02-hook-webview2.md); grep -qF 'TODO(windows)' <<<"$s" && grep -qF '| mode | presses_sent | pressed_logged | released_logged | first_gap_at | foreground_at_end |' <<<"$s" && s=$(awk '/^## Evidência medida/,0' docs/spikes/03-overlay-colagem.md) && grep -qF 'TODO(windows)' <<<"$s" && grep -qF '| cycles | overlay_visible | topmost | focus_kept | hidden_after_cancel | show_latency_ms_p50 | show_latency_ms_max |' <<<"$s" && grep -qF '| app | kind | repeat | restored | pasted |' <<<"$s"`

**C17** - `02-hook-webview2.md` records that the measured hook is `handy-keys` (`WH_KEYBOARD_LL` in `platform/windows/listener.rs`), that `rdev` is an unused dependency of `apps/desktop`, and that `SendInput` keys reach the hook with `LLKHF_INJECTED` (plan AC 17)
Proof: `f=docs/spikes/02-hook-webview2.md; for p in 'handy-keys' 'WH_KEYBOARD_LL' 'platform/windows/listener.rs' 'rdev' 'LLKHF_INJECTED'; do grep -qF -- "$p" $f || { echo "missing: $p"; exit 1; }; done`

**C18** - `03-overlay-colagem.md` cites the "tecla → pill visível ≤ 50 ms, máximo 100 ms" budget beside `show_latency_ms_*` and asks for a second paste round with `reliable_paste` = `true` (plan AC 18)
Proof: `f=docs/spikes/03-overlay-colagem.md; for p in '≤ 50 ms' '100 ms' 'show_latency_ms_' 'reliable_paste` = `true'; do grep -qF -- "$p" $f || { echo "missing: $p"; exit 1; }; done`

**C19** - `## Como reproduzir` of the paste report lists each app's text field: `data:text/html,<textarea>`, a new VS Code file, Teams' message box without sending, the Windows Terminal prompt, Notepad's body (plan AC 19)
Proof: `s=$(awk '/^## Como reproduzir/,/^## Evid/' docs/spikes/03-overlay-colagem.md); for p in 'data:text/html,<textarea>' 'VS Code' 'Teams' 'Windows Terminal' 'Bloco de Notas'; do grep -qF -- "$p" <<<"$s" || { echo "missing: $p"; exit 1; }; done`

### S5 - higiene · 3 files · 1 KB · ~1k

**C20** - The change touches no `.rs`, `Cargo.toml` or `Cargo.lock` (plan AC 20)
Proof: `test -z "$(git diff --name-only main...HEAD -- '*.rs' '*Cargo.toml' 'Cargo.lock')"`

**C21** - Each script starts with `#Requires -Version 5.1`, has a `param(` block and `Set-StrictMode -Version Latest`, and contains `handy` only inside `handy-keys` (plan AC 21, door 1)
Proof: `for f in scripts/spikes/*.ps1; do test "$(head -1 $f)" = '#Requires -Version 5.1' && grep -q '^param(' $f && grep -qF 'Set-StrictMode -Version Latest' $f && ! grep -io 'handy[^-]\|handy$' $f | grep -q . || { echo "$f"; exit 1; }; done; test "$(ls scripts/spikes/*.ps1 | wc -l)" -eq 3`

**C22** - `scripts/check-brand.sh` exits 0 (plan AC 22)
Proof: `scripts/check-brand.sh`

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| scripts (3) | `02-hook-count` C1-C6 · `03-overlay-focus` C7-C10 · `03-paste-check` C11-C13 | - |
| script exit statuses (3 per script) | hook: 0/1 C6, 2 C6 · overlay: 1 C9, 2 C10, 0 C9 · paste: 1 C13, 2 C13, 0 C13 | - |
| hook modes (2) | `auto` C2 · `manual` C4 | - |
| overlay counts (4) | `overlay_visible` C8, C9 · `topmost` C8, C9 · `focus_kept` C8, C9 · `hidden_after_cancel` C8, C9 | - |
| clipboard kinds (2) | `text` C12 · `image` C12 | - |
| apps (5) | chrome C11, C19 · Code C11, C19 · Teams C11, C19 · WindowsTerminal C11, C19 · notepad C11, C19 | - |
| reports (2) | `02-hook-webview2.md` C14-C17 · `03-overlay-colagem.md` C14-C16, C18, C19 | - |
| app configuration keys (5) | `--debug` C15 · `F9` push-to-talk C15 · `paste_method` C15 · `clipboard_handling` C15 · `reliable_paste` C15, C18 | - |
| doors (2) | door 1 script shape C21 · door 2 hook log count C3 | - |

- Every behavioural claim above is proven statically; its runtime proof is the `TODO(windows)` run
  in each report. Named, not hidden.

## Swept

- validation: C6, C10, C13 (exit 2 before any side effect)
- failure modes: C6, C9, C13 (exit 1 on any mismatch; a failed combination is a row, not an abort)
- idempotency: n/a - each run is a new measurement; the paste script restores the clipboard it set
- authorization: n/a - local scripts against the local app; no credentials
- concurrency: C3 (the log is read while the app writes and may rotate it)
- data lifecycle: n/a - the scripts persist nothing; the tables are copied into the reports by hand
- dependency failure: C6, C10, C13 (app not running, log missing, clipboard unreadable)
- state transitions: C8 (toggle → visible → cancel → hidden, per cycle)
- observability: C5, C9, C13 (stdout is only the table; progress and prompts on stderr)

## Handoff

- S1-S5: three new scripts (~20 KB) + two new reports (~12 KB) written, ≈ 40 KB of desktop
  sources read (`fala_keys.rs`, `lib.rs` single-instance and log setup, `overlay.rs`,
  `clipboard.rs`, `settings.rs`, tauri-plugin-log format) ≈ 72 KB / 4 ≈ 18k, under the 150k
  budget - one builder
- Mechanism: one builder (fits)
