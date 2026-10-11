# updater-on checks

Profile: light
Plan: `.specs/features/updater-on/plan.md`

16 checks in 3 slices · 4 one-way doors · 0 open

Prefixo de todo `cargo` abaixo: `CARGO_TARGET_DIR='C:\f\updater'` (Windows). Testes Rust ao lado do código em `apps/desktop/src/app_update.rs`.

## Checks

### S1 - núcleo em Rust (`feat/updater-core`)

**C1** - `install_refusal` devolve `Some(Dictation)`, `Some(MeetingRecording)` e `Some(PostProcessing)` com só a atividade correspondente ligada (as quatro de `Activity`: ditado, reunião gravando, transcrição de reunião, LLM), e `None` com tudo parado; com ditado e reunião ao mesmo tempo, recusa (AC 1)
Proof: `cargo test -p fala --lib app_update::tests::refuses_during_each_capture`

**C2** - `with_key_fallback` chama a segunda chave só depois de um erro de assinatura da principal; não chama com outro erro; com a segunda vazia devolve o erro da principal; com sucesso na principal, uma chamada só (AC 2, door 2)
Proof: `cargo test -p fala --lib app_update::tests::second_key_only_after_a_signature_failure`

**C3** - `offers_update("0.1.0", v)` é falso para `0.1.0` e `0.0.9`, verdadeiro para `0.1.1`; `0.2.0-rc.1` é oferecida a `0.1.9` e não a `0.2.0` (AC 3)
Proof: `cargo test -p fala --lib app_update::tests::only_a_newer_version_is_offered`

**C4** - `should_offer` esconde a versão dispensada numa checagem automática, oferece uma maior e oferece a dispensada numa checagem manual (AC 4)
Proof: `cargo test -p fala --lib app_update::tests::later_dismisses_that_version_only`

**C5** - `next_check_delay(seed)` fica em [6 h, 6 h 30 min] para sementes 0, 1, `u32::MAX` (AC 5)
Proof: `cargo test -p fala --lib app_update::tests::checks_every_six_hours_with_jitter`

**C6** - o template NSIS só roda `RunAsUser` quando `/NORELAUNCH` está ausente (door 3)
Proof: `grep -n 'NORELAUNCH' -A4 apps/desktop/nsis/installer.nsi` mostra o `${If} ${Errors}` em volta do `RunAsUser`

**C7** - `cargo clippy --workspace --all-targets -- -D warnings` e `cargo test -p fala --lib` saem com 0
Proof: os dois comandos

### S2 - ligar (`feat/updater-on`)

**C12** - `test_release_config.py` falha com a `pubkey` placeholder, com outro endpoint e com `createUpdaterArtifacts` falso, e passa com uma `pubkey` minisign bem formada (AC 6)
Proof: `python scripts/ci/test_release_config.py` (falha só em `test_updater_on` até a chave existir) e `-k test_guard`

**C13** - C31 de `shell.test.tsx` afirma `UPDATER_ENABLED === true`, o `UpdateChecker` e o `UpdateChecksToggle` montados em `AboutSettings.tsx`, e o toggle fora de `DebugSettings.tsx` (AC 7)
Proof: `bun src/components/shell.test.tsx`

**C14** - nada em `src/` importa `@tauri-apps/plugin-updater`; `capabilities/*.json` não tem `updater:` (AC 7, door 4)
Proof: `! grep -rn "plugin-updater" src && ! grep -rn '"updater:' apps/desktop/capabilities`

**C15** - `pickReleaseNote` (usado por `findReleaseNoteToShow` com a nota gravada) devolve a nota gravada quando a versão dela é a que roda e é maior que a última vista, e nada quando já foi vista (AC 11)
Proof: `bun src/components/whats-new/pickReleaseNote.test.ts`

**C16** - `bun run check:translations`, `bun run lint` e `bun run format:check` saem com 0; nenhuma chave `footer.` sobra nos locales (AC 12)
Proof: os três comandos + `! grep -n '"footer"' src/i18n/locales/*/translation.json`

**C10** - `new-updater-key.ps1` chama `tauri signer generate` com `-w`, lê só o `.pub`, grava em `plugins.updater.pubkey` e não tem `Write-Host`/`Write-Output` do conteúdo do arquivo privado (AC 9)
Proof: leitura do script + `python -X utf8 scripts/ci/test_release_config.py -k test_key_script_writes_only_the_public_key`

### S3 - ferramentas de release (`build/release-updater`)

**C8** - `gen-latest-json.py` com `.exe` e `.sig` válidos escreve `version`, `notes`, `pub_date` e as chaves `windows-x86_64` e `windows-x86_64-nsis` com a mesma URL `releases/download/vX.Y.Z/Fala_X.Y.Z_x64-setup.exe` e a assinatura do `.sig` (AC 8, door 1)
Proof: `python scripts/ci/test_gen_latest_json.py -k test_builds_both_platform_keys`

**C9** - recusa `.sig` ausente, `.exe` de outra versão e assinatura feita com outra chave, cada uma com motivo nomeado e sem escrever o arquivo (AC 8)
Proof: `python scripts/ci/test_gen_latest_json.py`

**C11** - `docs/RELEASE.md` "Construir" cita `TAURI_SIGNING_PRIVATE_KEY` e `_PASSWORD` e o `.sig`; "Publicar" cita `latest.json`, `gen-latest-json.py` e `--prerelease`; "Validar" cita atualizar a partir da versão anterior; "Updater" cita ADR-0018 e o feed; "Retirar" cita voltar para rascunho com o updater; `git diff` não toca "Versão e changelog" (AC 10)
Proof: `python scripts/ci/test_release_config.py` (testes de seção) + `git diff origin/feat/updater-on...HEAD -- docs/RELEASE.md`

## Fora da tabela (L-004)

- Ponta a ponta no Windows: instalar a versão anterior com updater, publicar a seguinte, ver a oferta, recusar durante um ditado, reiniciar para atualizar, conferir configurações e histórico, ver o "Novidades". Precisa da chave e de dois releases: passo do Augusto, listado no HANDOFF, nunca PASS aqui.
