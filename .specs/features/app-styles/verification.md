# app-styles — verification

Windows 11, `CARGO_TARGET_DIR=C:\f\app-styles`, 2026-10-12.

| Check | Comando | Resultado |
|---|---|---|
| C1 | `cargo test -p fala-postproc` | 32 + 27 + 1 passed (inclui `app_style_is_the_only_payload_change`) |
| C2 | `cargo test -p fala --lib -- app_styles:: llm_auto::tests::app_style` | 5 passed |
| C3, C4 | idem | `four_ready_styles…`, `styles_are_trimmed…`, `mappings_use_the_app_name…`, `instructions_follow_the_mapping` ok |
| C5–C7 | `bun src/components/settings/appStyles.test.tsx` | `C5 ok`, `C6 ok`, `C7 ok` |
| — | `cargo test -p fala --lib` | 509 passed, 0 failed, 1 ignored |
| — | `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| — | `cargo fmt --all -- --check`, `bun run lint`, `bunx prettier --check .`, `bunx tsc --noEmit`, `bun run check:translations` | exit 0 |
| — | `bun src/components/settings/cleanupLevel.test.tsx` | até `C11 ok` (tela de IA intacta) |

`src/bindings.ts` regenerado com `fala.exe --list-devices` (portátil): só as 39 linhas dos dois
comandos e dos dois tipos.

## Not tested

- Ditado real com a chave do Gemini (chamada paga): mapear `notepad → E-mail`, ditar mais de 15
  palavras no Bloco de Notas e ver o texto em parágrafos; trocar para `Mensagem` e ver o texto
  curto. Fica para o mantenedor.
- A tela aberta no app: o render foi provado só por `renderToStaticMarkup`; foco, blur e o
  `Select` não foram clicados.

## Revisão (`code-review high`)

Corrigidos: o painel de privacidade passa a citar a instrução do estilo; `style-N` novo não toma
o id de um estilo que vem depois; a linha de estilo ressincroniza com o que foi guardado e salva
aparado; o blur que salva não desabilita os botões no meio do clique; o estilo escolhido que foi
removido não habilita "Adicionar"; mapeamento de estilo removido não aparece; `ask` usa o estilo
que `process` resolveu. Mantidos, com o porquê no PR: a instrução do estilo no payload (o
invariante pedido é "só ganha a instrução do estilo"), nomes prontos em pt-BR (dado do usuário),
remover sem confirmação (como as outras listas da tela), normalização do nome do app no
frontend (só para trocar no lugar; o backend é a fonte).
