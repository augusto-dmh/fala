# updater-on

## Problem

A ADR-0018 (proposta) liga o updater antes da `v0.1.0`. O lado cliente herdado do Handy está no código e desligado: `UPDATER_ENABLED = false` em `apps/desktop/src/settings.rs` e `src/lib/updater.ts`, `plugins.updater` vazio e `createUpdaterArtifacts: false` em `tauri.conf.json`, travados por `scripts/ci/test_release_config.py` e pela C31 de `src/components/shell.test.tsx`. O `UpdateChecker` herdado está órfão (o rodapé saiu com a ADR-0017) e chama `downloadAndInstall` direto do frontend, sem portão: no Windows o NSIS fecha o app e uma gravação de reunião morre no meio.

## Flow

1. Ao abrir e a cada 6 h + jitter (0–30 min), `app_update` (novo, `apps/desktop`) checa `releases/latest/download/latest.json` se `UPDATER_ENABLED`, `update_checks_enabled` e não `FALA_DISABLE_UPDATER`, e se é um bundle instalado (não debug, não portátil).
2. Versão estritamente maior e não dispensada: baixa em segundo plano. `download` verifica a assinatura com a `pubkey` do config; se a verificação falha e `UPDATER_PUBKEY_NEXT` não é vazia, checa e baixa de novo com ela.
3. Bytes verificados ficam em memória; o status vira `ready`; a bandeja mostra "Reiniciar para atualizar" e a seção Sobre mostra a oferta (Reiniciar / Depois / Notas da versão).
4. Clique em "Reiniciar para atualizar" (bandeja ou Sobre) ou saída do app: `app_update::install` pergunta a `install_refusal` (a única decisão) se há ditado, reunião ou pós-processamento por LLM. Recusa com motivo nomeado; senão grava as notas para o "Novidades" e chama `Update::install` (NSIS passivo; ao sair, com `/NORELAUNCH`).
5. Depois do relançamento, o `WhatsNewGate` mostra as notas uma vez.
6. "Depois" grava `dismissed_update_version`; uma checagem automática não oferece mais aquela versão; uma versão maior ou "Verificar agora" oferece.

## Impact

| Front | What changes |
| --- | --- |
| domain | termo novo: `UpdateStatus` (fase, versão, progresso, dispensa, recusa), vive em `apps/desktop/src/app_update.rs` |
| domain | termo novo: `InstallRefusal` (`Dictation`, `MeetingRecording`, `PostProcessing`), idem |
| stored data | `AppSettings` ganha `dismissed_update_version: Option<String>` e `update_release_notes: Option<StoredReleaseNote>`, ambos `#[serde(default)]`; nada a migrar |
| config | `tauri.conf.json`: `createUpdaterArtifacts: true`, `endpoints` = feed, `pubkey` placeholder até o Augusto rodar `scripts/release/new-updater-key.ps1` |
| capability | o frontend perde `updater:default`: só o Rust baixa e instala |
| installer | `nsis/installer.nsi` aceita `/NORELAUNCH` para não reabrir o app quando o update roda ao sair |

## Surface

- Comandos specta: `update_status`, `check_for_updates_now`, `install_update`, `dismiss_update`; evento `UpdateStatus`. `trigger_update_check` e o evento `check-for-updates` saem.
- `scripts/release/gen-latest-json.py` (CLI do mantenedor) e `scripts/release/new-updater-key.ps1`.

## Landing

| One-way door | Literal shape | Alternative rejected |
| --- | --- | --- |
| 1. feed e chaves de plataforma | `https://github.com/augusto-dmh/fala/releases/latest/download/latest.json`; `platforms` com `windows-x86_64` e `windows-x86_64-nsis` apontando para o mesmo `.exe` | só a chave `-nsis`: um cliente que não manda o tipo de bundle não acharia nada |
| 2. segunda chave | `pub const UPDATER_PUBKEY_NEXT: &str = ""` em `app_update.rs`, tentada só quando a verificação com a principal falha | lista de chaves no `tauri.conf.json`: o schema do plugin tem uma `pubkey` só |
| 3. relançar ao sair | flag `/NORELAUNCH` no template NSIS, que pula o `RunAsUser` do `/R` | relançar sempre: quem saiu do app o veria reabrir |
| 4. instalar só pelo Rust | o frontend não tem `updater:default`; `install_update` é o único caminho | manter `check()`/`downloadAndInstall` no JS: o portão viraria opcional |

## Criteria

1. `install_refusal` recusa com `Dictation`, `MeetingRecording` e `PostProcessing` quando a atividade correspondente está ligada, e aceita só com tudo parado (ADR-0018 §3).
2. Uma falha de assinatura com a chave principal tenta a segunda chave quando ela existe; outra falha não tenta; com a segunda vazia, o erro é o da principal.
3. Só uma versão estritamente maior é oferecida (sem downgrade; `0.2.0-rc.1` < `0.2.0`).
4. A versão dispensada não é oferecida por checagem automática; uma maior é; "Verificar agora" ignora a dispensa.
5. O intervalo entre checagens fica em [6 h, 6 h 30 min].
6. `test_release_config.py` exige `pubkey` minisign válida, `endpoints` igual ao feed e `createUpdaterArtifacts` ligado, e continua barrando `certificateThumbprint`, `signCommand` e `tsp`.
7. `UPDATER_ENABLED` é `true` nos dois lados; o `UpdateChecker` e o toggle aparecem na seção Sobre; o toggle diz que a checagem consulta o GitHub; nada em `src/` importa `@tauri-apps/plugin-updater`.
8. `gen-latest-json.py` monta o manifesto com as duas chaves de plataforma e recusa `.sig` ausente, nome de outra versão e assinatura que não verifica contra a `pubkey`.
9. `new-updater-key.ps1` grava só a chave pública no config e não imprime a privada.
10. `docs/RELEASE.md` cobre as variáveis de assinatura, o `.sig`, o `latest.json`, o teste de atualizar a partir da versão anterior e a retirada com updater; a seção "Versão e changelog" não muda.
11. Depois de um update, as notas gravadas antes de instalar aparecem uma vez no "Novidades".
12. Strings novas em pt e en; as `footer.*` herdadas saem do Title Case e do namespace `footer`.

## Slices (PRs empilhados sobre `docs/adr-0018-updater`)

- S1 `feat/updater-core`: `app_update.rs`, bandeja, saída, portão, rotação, settings, NSIS, bindings. Constantes ainda `false`: nada muda para quem usa. AC 1–5.
- S2 `feat/updater-on`: config, constantes, UI em Sobre, "Novidades", travas invertidas, strings, `new-updater-key.ps1`. AC 6, 7, 9, 11, 12. PR em draft até existir a chave.
- S3 `build/release-updater`: `gen-latest-json.py` + teste no CI, `RELEASE.md`. AC 8, 10.

## Assumptions

- `Update::download` verifica a assinatura antes de devolver os bytes e `Update::install` no Windows roda o NSIS com `/P /R /UPDATE` e chama `exit(0)` (lido em `tauri-plugin-updater` 2.10.1, `updater.rs:652-720`, `config.rs:36-45`).
- O ditado inteiro (gravação, transcrição, LLM, cola) é `TranscriptionCoordinator::is_busy()`; a edição tardia do LLM roda depois e ganha um contador próprio; notas e transcrição de reunião estão em `pipeline::Work`.
- O teste ponta a ponta (instalar a anterior e atualizar) só roda com uma chave e dois releases: fica fora da verificação desta máquina, como passo do Augusto (L-004: linha Unproven fora da tabela, sem virar PASS).
