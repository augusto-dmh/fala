# spike-windows-prep verification

**Verdict**: PASS
**Profile**: light
**Diff range**: 773cea7..3076466
**Round**: 2 - scoped
**Verifier**: independent sub-agent (author != verifier)

All 22 proofs exit 0 at `3076466`. Round 1 failed C3 because each poll opened and disposed a
fresh handle, so lines could be lost across a rotation. The rewritten tail keeps one handle open
and drains it before switching, and on Windows 11 with NTFS it delivers the claim. That depends
on one OS assumption, stated in the C3 row and under gap 1. This round re-judged C2, C3, C6, C8,
C10, C12, C13 and C15-C19 in full, and refreshed the citations for all five touched files. Rows
marked `carried from round 1` rest on round 1's reading at `41682da`. Their files did change, so
their citations were refreshed here, but no finding in them was re-judged.

## Binding sources

Carried from round 1. Profile `light`: step 1 does not run, and the plan marks no source as
binding. No contradiction or uncovered element was assessed.

## Checks

Verified at `3076466`. Each proof was extracted verbatim from `checks.md` with awk, run with
`bash` from the worktree root, and its exit code recorded. The proofs are literal greps, so each
row also cites the line that settles the claim, judged from the surrounding logic.

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C1 | hook script params and defaults | verbatim, exit 0 | `scripts/spikes/02-hook-count.ps1:21-28`: `[ValidateSet('auto', 'manual')][string]$Mode = 'auto'`, Presses 300, IntervalMs 2000, HoldMs 50, Minutes 10, ExpectedPresses, LogDir `$env:LOCALAPPDATA\br.com.augusto.fala\logs`, WindowTitle 'Fala'. Carried from round 1, citations refreshed. | PASS |
| C2 | auto: activate, SendInput F9 down/up, hold, interval | verbatim, exit 0 | `scripts/spikes/02-hook-count.ps1:144` `[W]::SetForegroundWindow($hwnd)`. `:145-148`: a 300 ms sleep, then `if ([W]::GetForegroundWindow() -ne $hwnd) { Fail2 ... }`, so activation is now checked before any key is sent (round-1 gap 5 closed). `:152-154`: `[W]::Key(0x78, $false)`, `Start-Sleep -Milliseconds $HoldMs`, `[W]::Key(0x78, $true)`. `:156` `AddMilliseconds($IntervalMs)`. `:59-65`: SendInput with type 1 and KEYUP 2; it throws on `!= 1`. | PASS |
| C3 | counts only post-start lines; reads as it grows so a rotation mid-run loses no lines | verbatim, exit 0 | **Post-start:** `:98` sets the offset to the file length at start; `:108` seeks there. **Persistent handle:** `:106-107` opens once (`:139`) with `FileShare]::ReadWrite -bor ...Delete`, and `:112` reads with `ReadToEnd()`. It is never disposed between polls, so lines written during the 50 ms hold (`:153`) are read on the next poll. **Complete lines only:** `:114-117` counts up to the last `\n` and carries the rest in `pending`. **Rotation:** `:127` detects `(Get-Item ...).Length -lt $script:offset`. `:128` drains the old handle again. `:129-132` disposes it and resets the offset and pending. `:135-136` opens the new file at 0 and reads it. The app side matches this order: tauri-plugin-log 2.8.0 `lib.rs:206-209` flushes and drops the writer, `:222` calls `fs::remove_file`, and `:226` reopens with create and append. Every line in the old file is written before the new file exists, and the drain at `:128` runs after the new file is seen, so nothing written before the rotation is lost. **Assumption:** Rust `remove_file` is `DeleteFileW` (std `sys/fs/windows.rs:1299`). The old handle stays readable and the app can recreate the name only under POSIX delete semantics, which are the NTFS default on current Windows 10/11 builds. That holds for the Windows 11 target and was not verified on a machine here (gap 1). The length check also assumes `Get-Item` returns the current size (gap 2). | PASS |
| C4 | manual: stderr instruction, wait Minutes, compare ExpectedPresses | verbatim, exit 0 | `:169` stderr "digite na janela do Fala e toque F9 a cada ~10 s"; `:170` `AddMinutes($Minutes)`; `:168` `$sent = $ExpectedPresses`; `:186` compare. Carried from round 1, citations refreshed. | PASS |
| C5 | stdout is one table with the 6 columns | verbatim, exit 0 | `:182-184` header, separator and one row through `[Console]::Out`. The rotation notice (`:134`) and progress (`:149`, `:163`) go to `[Console]::Error`, and `Seek` and `SetForegroundWindow` output is piped to `Out-Null` (`:108`, `:144`). | PASS |
| C6 | mismatch exits 1; missing log or window exits 2 before the first SendInput | verbatim, exit 0 | `:83` and `:87` `Fail2` (exit 2, `:76-79`) come before `:152`, the first `[W]::Key(`. The new foreground `Fail2` at `:147` also comes before it. The only earlier effects are `Add-Type`, opening a read handle (`:139`) and the activation attempt (`:144`), and none sends input. `:186` `if ($script:pressed -ne $sent -or $script:released -ne $sent) { exit 1 }`. | PASS |
| C7 | overlay params and defaults | verbatim, exit 0 | `scripts/spikes/03-overlay-focus.ps1:18-21`: `[int]$Cycles = 200`, `[string]$FalaExe = 'fala.exe'`, `[int]$SettleMs = 500`, `[int]$PollMs = 10`. Carried from round 1, citations refreshed. | PASS |
| C8 | per cycle: save fg, toggle, poll Recording visible, WS_EX_TOPMOST, compare fg, cancel, wait hidden | verbatim, exit 0 | In order: `:84` `[Overlay03]::GetForegroundWindow()`; `:85` `Invoke-Fala '--toggle-transcription'`; `:86` `Wait-Overlay $true`, where `:73-74` calls `[Overlay03]::FindWindowW([NullString]::Value, 'Recording')` and `IsWindowVisible`, polled every PollMs up to SettleMs; `:91-92` `GetWindowLongPtr($shown.Hwnd, -20)` with `-band 0x8`; `:94` foreground compare; `:95` `--cancel`; `:96` `Wait-Overlay $false`. The class is now `Overlay03` (`:31`), different from `W` in 02 (round-1 gap 2 closed). `:36-43` adds the 32-bit fallback: `IntPtr.Size == 8 ? GetWindowLongPtrW : GetWindowLongW`, both returning `long`, which is valid C#. | PASS |
| C9 | 7-column table; any count below cycles exits 1 | verbatim, exit 0 | `:110-112` table; `:114` `if ($visibleCount -lt $Cycles -or $topmost -lt $Cycles -or $focusKept -lt $Cycles -or $hidden -lt $Cycles) { exit 1 }` | PASS |
| C10 | fala.exe not runnable, or Recording never visible in 3 cycles, exits 2 naming the cause | verbatim, exit 0 | `:52-53` `Get-Command $FalaExe`, then `Fail2 "não achei ..."`. `:57-58` is the new preflight: `Get-Process -Name <exe basename>` or `Fail2 "o Fala não está rodando ..."`. Both run before the first `Invoke-Fala` (`:85`). `:63-66` bounds each call with `WaitForExit(10000)`, then `Kill()` and `Fail2 "fala.exe $flag não voltou em 10 s"`, so round-1 gap 3 (a hang) is closed. `:97-98` `$i -eq 3 -and $visibleCount -eq 0` leads to `Fail2 "... nunca ficou visível ..."`. The 10 s `Fail2` can fire mid-run, after side effects and without a table (gap 3). | PASS |
| C11 | paste params and defaults | verbatim, exit 0 | `scripts/spikes/03-paste-check.ps1:20-26`: the Apps default, `[int]$Repeats = 3`, Kinds `@('text', 'image')`, `[string]$FalaExe`, Phrase `'teste um dois três'`, `[int]$SpeakSeconds = 4`, `[int]$PasteWaitMs = 3000`. Carried from round 1, citations refreshed. | PASS |
| C12 | per combination: guid text or 8x8 bitmap alone; 5 s countdown; toggle around SpeakSeconds; wait; compare; ask y/n | verbatim, exit 0 | `:108-109` `'FALA-SPIKE-' + [guid]::NewGuid()` and `SetText`; `:112-113` and `:78` `New-Object System.Drawing.Bitmap 8, 8` with `SetImage`; `:115-119` stderr "foque" and 5 x 1 s; `:120-124` toggle, `Start-Sleep -Seconds $SpeakSeconds`, toggle, `Start-Sleep -Milliseconds $PasteWaitMs`; `:90` text `-ceq`; `:92-97` image: no text, same size, `LockBits` Format32bppArgb bytes compared with `SequenceEqual`; `:127-129` `Read-Host` y/n loop. New: `:106`/`:131-135` wraps each combination in `try`/`catch`, and a failure sets `restored = 'n'` and `answer = 'n'` and the loop continues (round-1 gap 4 closed). The `Read-Host` prompt is host output, not stderr (precision, gap 5). | PASS |
| C13 | row table and total; any n exits 1; not runnable or clipboard unreadable exits 2 before the first combination | verbatim, exit 0 | `:144-147` header, rows, then `total`; `:149` `if ($restoredYes -lt $total -or $pastedYes -lt $total) { exit 1 }`. `:39` (Get-Command), `:44` (the new "Fala não está rodando" preflight) and `:51` (STA plus `ContainsText` probe) all `Fail2` before `:102` `foreach ($app in $Apps)`. `Invoke-Fala` (`:54-60`) now throws after 10 s, and the per-combination catch turns that into an n/n row, so a failed combination lands in the table and exit 1 follows. | PASS |
| C14 | both reports have the 3 headings, none with Recomenda | verbatim, exit 0 | `docs/spikes/02-hook-webview2.md:5`, `:29`, `:65`; `docs/spikes/03-overlay-colagem.md:5`, `:23`, `:67`. "não recomenda" appears only in body text (`02:27`, `03:21`). | PASS |
| C15 | Como reproduzir names the build doc, --debug, F9 PTT, CtrlV, DontModify, reliable_paste and the invocations | verbatim, exit 0 | `02-hook-webview2.md:31` build doc, `:34` F9 push-to-talk, `:35` CtrlV/DontModify/`reliable_paste` = `false`, `:37` `--debug`, `:46` `... 02-hook-count.ps1 -Mode auto -Presses 300 -IntervalMs 2000 -HoldMs 50`. `03-overlay-colagem.md:25`, `:27-30`, `:38` `-Cycles 200 -SettleMs 500 -PollMs 10`, `:49` `03-paste-check.ps1 -Apps chrome,Code,Teams,WindowsTerminal,notepad -Repeats 3 -Kinds text,image`. The invocations now go through `pwsh -ExecutionPolicy Bypass -File` (round-1 gaps 6 and 7 are addressed in the docs, `02:40-42`, `03:31-34`). | PASS |
| C16 | Evidência medida has TODO(windows) and the exact empty tables | verbatim, exit 0 | `02-hook-webview2.md:67`, `:70`; `03-overlay-colagem.md:69`, `:75`, `:81`, `:87`. The headers are byte-equal to the script headers (`02-hook-count.ps1:182`, `03-overlay-focus.ps1:110`, `03-paste-check.ps1:144`). | PASS |
| C17 | 02 report: handy-keys hook in listener.rs, rdev unused, LLKHF_INJECTED | verbatim, exit 0 | `02-hook-webview2.md:15-18`, `:22-25`. That section did not change. The facts were checked against the code in round 1 (handy-keys `listener.rs:367`, `apps/desktop/Cargo.toml:68`), and that check is carried from round 1. | PASS |
| C18 | 03 report cites the 50/100 ms budget beside show_latency_ms_*, asks for a reliable_paste=true round | verbatim, exit 0 | `03-overlay-colagem.md:62-65` and `:72-73` place the budget "≤ 50 ms, máximo 100 ms" beside `show_latency_ms_p50`/`_max`; `:59-60` and `:85` ask for a second round with `reliable_paste` = `true`. | PASS |
| C19 | paste report lists each app's text field | verbatim, exit 0 | `03-overlay-colagem.md:42-46`: Chrome `data:text/html,<textarea>`, VS Code new file, Teams message box without sending, Windows Terminal prompt, Bloco de Notas body | PASS |
| C20 | no .rs / Cargo.toml / Cargo.lock touched | verbatim, exit 0 | `git diff --name-only main...HEAD` lists only `checks.md`, the two `docs/spikes/*.md` files and the three `scripts/spikes/*.ps1` files | PASS |
| C21 | each script: #Requires 5.1 first line, param(, StrictMode Latest, handy only in handy-keys | verbatim, exit 0 | `02-hook-count.ps1:1`, `:20`, `:31`; `03-overlay-focus.ps1:1`, `:17`, `:24`; `03-paste-check.ps1:1`, `:19`, `:29`; exactly 3 scripts | PASS |
| C22 | check-brand.sh exits 0 | verbatim, exit 0, prints `ok: no Handy branding outside the allowlist` | `scripts/check-brand.sh` run at `3076466` | PASS |

## Report fact check

The edited sections were verified at `3076466`. The unchanged facts are carried from round 1.

- `02:52-57` says the script keeps one handle open and drains the old file before opening the
  new one. The script does that (`02-hook-count.ps1:105-137`). The parenthetical "no Windows 10+
  a exclusão tira o nome, mas o handle segue lendo" overstates the OS support. POSIX delete
  semantics only became the `DeleteFileW` default in a later Windows 10 feature update, and only
  on NTFS. It holds on Windows 11. This is a precision gap (gap 1), not a check claim.
- `02:59-60` says auto mode exits 2 when the window cannot be brought forward. True
  (`02-hook-count.ps1:144-148`).
- `03:32-34` says both scripts exit 2 without a running instance. True
  (`03-overlay-focus.ps1:57-58`, `03-paste-check.ps1:43-45`).
- `03:55-56` says a failed combination (busy clipboard, `fala.exe` silent for 10 s) becomes an
  n/n row and the matrix continues. True (`03-paste-check.ps1:105-135`, `:56-59`).
- `03:31-32` and `02:40-41` say 5.1 reads the scripts as ANSI. True: all three files start
  `23 52 65` (`#Re`), with no BOM.
- The facts carried from round 1 are unchanged in the source: `fala_keys.rs:130-133`,
  `lib.rs:796-797`, `lib.rs:852-858`, `overlay.rs:153-171` and `:409-421`, `clipboard.rs:64-68`,
  and `clipboard.rs:812-834`.

## Swept rows

Verified at `3076466`. No `Swept` row resolves to `existing`. Round 1 found the `failure modes`
row (`checks.md:113`, "a failed combination is a row, not an abort") untrue of the paste script.
It is now true (`03-paste-check.ps1:106-135`). The `concurrency` row (C3) holds under the
assumption in gap 1.

## PowerShell 5.1 and StrictMode

Re-checked in the changed code at `3076466`.
- No PowerShell 7-only syntax: a search for `??`, `?.`, `&&`, `||` and ternaries finds only the
  C# ternaries inside the `Add-Type` sources (`02-hook-count.ps1:63`,
  `03-overlay-focus.ps1:42`).
- The `-bor` inside the `File.Open` argument list (`02-hook-count.ps1:106-107`) parses as one
  argument.
- `New-Object System.IO.StreamReader($fs, UTF8)` binds to `StreamReader(Stream, Encoding)`.
- `.Split("`n")` works on .NET Framework (char[]) and on .NET (string).
- `WaitForExit(int)` exists in both runtimes.
- `$x = if (...) {...} else {...}` (`03-paste-check.ps1:126`) is valid in 5.1.
- Under StrictMode Latest, every `$script:` variable is initialized before use
  (`02-hook-count.ps1:98-104`). `$restored` and `$answer` are assigned on both the try and catch
  paths (`03-paste-check.ps1:126-134`). The hashtable keys `Ms`/`Hwnd` always exist
  (`03-overlay-focus.ps1:75`, `:78`).
- `pwsh -File` starts one process per script, so an `Add-Type` name collision cannot recur. The
  type names also differ now (`W`, `Overlay03`).

## Round-1 gaps: status at 3076466

| Round-1 gap | Status |
| --- | --- |
| 1 C3 rotation loses lines | closed, subject to the assumption in gap 1 below |
| 2 class `W` defined twice | closed (`03-overlay-focus.ps1:31` `Overlay03`) |
| 3 03 scripts hang without Fala | closed (`03-overlay-focus.ps1:57-58`, `:63-66`; `03-paste-check.ps1:43-45`, `:56-59`) |
| 4 one exception aborts the paste matrix | closed for the paste script (`03-paste-check.ps1:106-135`) |
| 5 `SetForegroundWindow` unchecked | closed (`02-hook-count.ps1:145-148`) |
| 6 UTF-8 without BOM under 5.1 | documented, not fixed (`02:40-41`, `03:31-32`); 5.1 still shows mojibake |
| 7 execution policy | closed in the docs (`-ExecutionPolicy Bypass`, `02:46`, `02:49`, `03:38`, `03:49`) |
| 8 precision gaps | 32-bit `GetWindowLongPtrW` closed (`03-overlay-focus.ps1:36-43`); the rest remain (gap 5) |
| 9 level gap by design | remains (gap 6) |

## Gaps (ranked)

1. **C3 depends on POSIX delete semantics.** The script holds `fala.log` open across the run
   (`02-hook-count.ps1:106-107`, `:139`). When the app rotates, it deletes the file with
   `DeleteFileW` (tauri-plugin-log `lib.rs:222`, Rust std `sys/fs/windows.rs:1299`) and
   recreates it with create+append (`lib.rs:226`, `:196-199`). Under POSIX semantics, the NTFS
   default on current Windows 10/11 builds, the name goes away, the old handle stays readable,
   and the new file opens. The claim holds there. Under legacy semantics (older Windows 10
   builds, or a non-NTFS volume) the file would stay delete-pending while the handle is open.
   The app's `open_file` would then fail and it would stop logging until the script exits, which
   would show up as a false hook miss (exit 1). The report's "no Windows 10+"
   (`02-hook-webview2.md:55-56`) overstates this. Nothing on this machine could verify it; the
   first Windows run shows it, through the stderr rotation notice (`:134`) and a count that
   keeps rising after it.
2. **Rotation detection trusts the size read by path.** `02-hook-count.ps1:127` compares
   `(Get-Item).Length` with the offset read through the handle. A size that lags behind the
   writer would look like a recreated file. The script would then reopen at 0 and count the
   whole file again, reporting more presses than were sent. `GetFileAttributesEx`, which
   `FileInfo` uses, is expected to return the live size, so the risk is low but unverified.
3. **Mid-run failures in the 03 scripts.** `03-overlay-focus.ps1:63-66` exits 2 after cycles
   have run, prints no table, and may leave a recording on. In `03-paste-check.ps1:120-123`, a
   caught failure between the two toggles can leave the recording state inverted for later
   combinations. Those rows would read `n`, so no false pass results. `$p.Kill()`
   (`03-overlay-focus.ps1:64`, `03-paste-check.ps1:57`) throws if the process exits between
   the timeout and the kill.
4. **`[W]::Key` throws on a refused SendInput** (`02-hook-count.ps1:64-65`), for example when
   UIPI blocks it. With `$ErrorActionPreference = 'Stop'` (`:32`) that aborts without a table.
   Carried from round 1's reading and still present.
5. **Precision gaps that remain.**
   - `first_gap_at` notices only a missing Pressed (`02-hook-count.ps1:161`), while AC 3 says
     "sem par".
   - The `Read-Host` prompt is host output, not stderr (`03-paste-check.ps1:128`).
   - The 3-cycle exit 2 never fires when `-Cycles` is below 3 (`03-overlay-focus.ps1:97`).
   - `hidden_after_cancel` counts cycles where the pill never appeared (`:96`).
   - The 500 ms `-SettleMs` leaves about 200 ms over the app's 300 ms delayed hide.
   - Under 5.1 the messages and the displayed `-Phrase` are mojibake (no BOM).
6. **Level gap, by design.** Every proof is a literal grep. Runtime behaviour, including gaps 1
   and 2, stays unproven until the `TODO(windows)` run.

## Gate

`python3 /home/augusto/.claude/skills/tlc-spec-lean/scripts/validate_verification.py --root /home/augusto/projects/fala-windows-prep spike-windows-prep`. exit 0: "0 error(s), 0 warning(s) across [spike-windows-prep]". 22 proofs run at `3076466`, all exit 0, and 22/22 checks PASS.
