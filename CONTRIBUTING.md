# Contribuindo com o Fala

Este é o documento canônico de fluxo, commits e pull requests. `AGENTS.md` só o resume. Por que cada regra existe: `fala-research/research/12-convencoes-de-commits-e-prs.md`.

## Fluxo

1. `git switch -c <type>/<slug>` a partir de `main` atualizado. Nunca commite em `main`.
2. Commits pequenos na branch; `git pull --rebase origin main` para atualizar, nunca merge de `main` para dentro.
3. `gh pr create` com o template preenchido; CI verde; leitura do diff completo.
4. Squash pelo mantenedor: o **título do PR vira o commit em `main`** e o corpo vira o corpo do commit.
5. A branch é apagada no merge; remova o worktree.

Uma preocupação por PR. Se a descrição diz "also", divida.

Rode `bun install && bunx lefthook install` uma vez por clone: os hooks barram commit e push fora da regra; o CI repete as mesmas verificações.

## Commits

Formato [Conventional Commits 1.0.0](https://www.conventionalcommits.org/en/v1.0.0/), verificado pelo commitlint no `commit-msg`:

```
<type>(<scope>): <description>

<body: the problem, then why this change; what changes in behaviour>

Refs: #12
```

- **Tipos:** `build` `chore` `ci` `docs` `feat` `fix` `perf` `refactor` `revert` `style` `test`. Nada fora disso (sem `refac`, sem `release`; o release é `chore(release): v1.2.3`).
- **Escopo** opcional, kebab-case: nome do crate (`asr`, `hotkey`, `desktop`, `cli`) ou área do frontend (`ui`, `i18n`). Sem escopo quando a mudança atravessa áreas sem uma dominante.
- **Descrição** em inglês, minúscula, imperativo, sem ponto final, cabeçalho ≤ 72 caracteres. Teste: "If applied, this commit will _description_".
- **Corpo** obrigatório para `feat`, `fix`, `perf`, `refactor` e breaking: explica o porquê, não o quê (o diff mostra o quê). Linhas ≤ 100.
- **Breaking:** `!` antes de `:` ou rodapé `BREAKING CHANGE:`.
- **Rodapés** só em formato trailer: `Refs: #N`, `Closes #N`, `Assisted-by:`. Sem `Co-Authored-By`, sem "Generated with", sem link de sessão. Sem `Signed-off-by` (não há DCO).
- Um assunto por commit; testes no mesmo commit que o comportamento; fixups via `--amend` ou `rebase -i` antes do PR, não como commits "fix lint".
- Cite só o que vive no repo: `docs/decisions/NNNN`, `#issue`, fases do `ROADMAP.md`. Nunca caminhos de spec locais nem IDs internos de tarefa do skill.

Bons: `feat(inject): restore the clipboard after pasting`, `fix(hotkey): release fires when the window loses focus`. Ruins: `fix bug`, `update docs`, `Update main.rs`.

## Pull requests

- **Título** = cabeçalho de commit, mesmas regras. Para `feat`, `fix` e `perf`, na voz de quem usa: o que muda para a pessoa, não o que o código faz (`feat(dictation): paste stays in the app that had focus`, não `use SendInput instead of enigo`).
- **Corpo** com as quatro seções do template, em inglês, sem checklist e sem os comentários do template: **Problem**, **Change**, **Verification** (comandos exatos, contagens de testes, o que não foi testado; captura ou GIF para UI), **AI assistance**. `Closes #N` vai no corpo.
- **Tamanho:** alvo ≤ 400 linhas alteradas; o CI falha acima de 1.000 sem a label `large-change`, que precisa de justificativa no corpo. Lockfiles, traduções e binários não contam. Feature maior vira PRs empilhados (branch B a partir de A; `gh pr edit --base main` quando A mergear).
- Comentário de revisão que muda o escopo abre issue; não infla o PR.
- Versão e `CHANGELOG.md` não mudam em PR de feature. O release é do mantenedor: `git cliff` gera a entrada a partir dos commits de `main`, o commit é `chore(release): vX.Y.Z`, por PR como qualquer outro.

## Uso de IA

Bem-vindo, com transparência. Você é responsável pelo que envia e precisa entender cada linha; se não consegue explicar o que a mudança faz, não a envie. Sem respostas de revisão copiadas de IA; sem agentes autônomos abrindo PRs.

- Todo commit feito por agente leva o trailer `Assisted-by: <ferramenta>` (o Claude Code o gera a partir de `.claude/settings.json`). Commits escritos à mão não levam trailer.
- A seção **AI assistance** do PR descreve extensão e propósito ("the agent wrote the code and tests from the plan; I reviewed the diff and ran the smoke test on Windows"), revisada por um humano antes do merge.

## Licença

Contribuições entram sob a licença MIT do projeto (`LICENSE`). Não há CLA.
