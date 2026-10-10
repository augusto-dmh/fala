# ui-onboarding verification (S3, PR 3)

**Verdict**: PASS
**Profile**: light
**Diff range**: 4163d34..e986059
**Round**: 1 - full
**Verifier**: independent sub-agent (author != verifier)

Escopo: slice S3 (C17–C19, decisão delegada D12), verificado em `e986059` (`docs/adr-ui-identity`). S1 e S2 têm relatórios próprios.

## Passos que não rodam em `light`

- Passo 1 (checks contra as fontes vinculantes): não roda em `light`. Mesmo assim, a leitura de revisor pedida no brief está em "Achados".
- Recomputação do `Coverage`: não roda em `light`.
- Passo 4 (injeção de falhas): não roda em `light`. A discriminação de C18 pedida no brief rodou e está registrada abaixo.

## Checks

| Check | Claim | Proof run | Evidence | Result |
| --- | --- | --- | --- | --- |
| C17 | ADR-0017 `proposed`, `date: 2026-10-10`, seções do template, três regras, cita `docs/design/ui.md`; nenhum `status: accepted` novo | `bun src/styles/tokens.test.ts` exit 0, imprime `C17 ok` | `src/styles/tokens.test.ts:310` - `assert.match(adr, /^---\nstatus: proposed\ndate: 2026-10-10\n---\n/)`; `:311-318` - loop `adr.includes(\`\n${heading}\n\`)` sobre as 5 seções; `:320`, `:321`, `:323-325` - as três regras em negrito; `:326` - `adr.includes("\`docs/design/ui.md\`")`. Cláusula "nenhum accepted novo": `git diff 4163d34..e986059 -- docs/decisions` só tem `+status: proposed` (arquivo novo `0017-*`), nenhuma linha `-status` | PASS |
| C18 | `ui.md`: 13 tokens com tema + `rec` + `me` iguais ao `theme.css`; rampa 12/16…28/36; raios 4 e 8 px; janela 960×640, mín. 720×520, rail 220/48 | `bun src/styles/tokens.test.ts` exit 0, imprime `C18 ok` | `src/styles/tokens.test.ts:339-347` - `assert.deepEqual(rows.get(token), [rootBlock.get(\`--light-color-${token}\`), rootBlock.get(\`--dark-color-${token}\`)])` sobre `THEMED` (13, `:59`); `:350` - `rows.get("rec")` = `[rec, rec]`; `:351` - `designDoc.includes(...)` da linha `me` = `ok` nas duas colunas; `:352-354` - os 5 degraus (substring da célula de cada degrau); `:355-356` - raios; `:357` - `960×640 por padrão, mínimo 720×520`; `:358` - `220 px, recolhido em 48 px`. Discriminação: em worktree descartável, `text-2` claro `#5c5c5c`→`#5c5c5d` no `ui.md` fez o teste falhar em `:340` (exit 1); `danger` escuro `#ff99a4`→`#ff99a5` falhou com `ui.md danger matches theme.css` (exit 1) | PASS |
| C19 | PR 3 só muda `docs/` e o teste de tokens; portões verdes | `git diff --name-only 4163d34..e986059` lista exatamente 3 arquivos; `bunx prettier --check .` exit 0; `bun run format:check` exit 0; `bun run lint` exit 0; `scripts/check-brand.sh` exit 0 (`ok: no Handy branding outside the allowlist`); T1 exit 0 | saída do `git diff --name-only`: `docs/decisions/0017-identidade-visual-e-navegacao-da-ui.md`, `docs/design/ui.md`, `src/styles/tokens.test.ts` (3 files, 203 insertions, 0 deletions) | PASS |

## Discriminação de C18 e higiene da árvore

- `git worktree add --detach <scratchpad>/verify-s3 e986059`, `node_modules` por symlink da worktree real; mutações só no scratch.
- Mutante 1 (`ui.md`, `text-2` claro): morto, `AssertionError` em `tokens.test.ts:340`, exit 1.
- Mutante 2 (`ui.md`, `danger` escuro): morto, `ui.md danger matches theme.css`, exit 1.
- Mutante extra (ADR-0017 `status: proposed`→`accepted`): morto em C17 (`:310`).
- Scratch removido (`git worktree remove --force` + `prune`); `git status --porcelain` da árvore real antes e depois idênticos (vazio), HEAD continua `e986059`.

## D12 e numeração

- `ls docs/decisions`: 0000–0016 e a nova 0017; 0017 é o próximo número livre.
- `git fetch` + `git ls-tree` em todas as 213 branches remotas: nenhuma tem `docs/decisions/0017*`. Entre as branches locais, só `docs/adr-ui-identity`. Entre as worktrees, só esta.
- Linhas `status:` de `docs/decisions/` em HEAD: as ADRs 0010, 0011, 0012 e 0017 estão `proposed`, a 0003 está `superseded by ADR-0009` e as outras estão `accepted`, como na base (o diff não toca nenhum arquivo existente).

## Achados (leitura de revisor)

Nenhum achado bloqueante.

- Regras da §3.11: a ADR traz exatamente as três (abre no conteúdo; cor só semântica, sem cor de marca; tokens em `src/styles/theme.css` como fonte única) e manda os valores para o `docs/design/ui.md`, que pode mudar sem ADR nova. Isso é a proposta da §3.11, com o caminho completo `src/styles/theme.css` no lugar de `styles/theme.css`.
- Valores do `ui.md` comparados com `theme.css` e §3.7: os 13 hex com tema, `rec #e5322d` e `me = ok` batem com `src/styles/theme.css:11-46,65` e com a tabela da §3.7 (só muda a caixa das letras). A rampa e os raios batem com `src/App.css:48-57,72-75`. A janela bate com `apps/desktop/src/lib.rs:960-961`, e o rail 220/48 abaixo de 840 px bate com `src/components/Rail.tsx:87,97` e com a §3.4. A lista de aliases inclui `warning`/`error`, que existem em `theme.css:79-80`. O `focus` da §3.7 aparece em prosa e não na tabela, o que não contradiz nada.
- O `ui.md` traz alguns valores que não estão na §3.7/§3.4: 100 ms padrão, `prefers-reduced-motion` e ícones de 20 px com traço 1,5. Conferi no código: `App.css:77,114` e `Rail.tsx:75`. Não contradizem a fonte e não são achado.
- Confirmação da ADR: cada mecanismo citado existe. Hex rosa em `tokens.test.ts:124-126`, contraste ≥ 4,5 em `:146`, abrir em Início em `src/components/shell.test.tsx:120` (`useState<RailDestination>("home")`).
- Conflito com ADR aceita: nenhum. A ADR-0001 (`:21`) manda trocar toda a marca do Handy, e a 0017 vai no mesmo sentido.
- Citações proibidas: a ADR, o `ui.md` e a mensagem do commit não citam caminho de spec local nem ID interno de tarefa. O comentário novo em `tokens.test.ts:298-299` cita `.specs/features/ui-onboarding/checks.md`, como o cabeçalho já existente do mesmo arquivo (`:1`) e dos outros testes de UI. `.specs/` é versionado, e a regra do AGENTS.md vale para commits e PRs, então não é achado. O `ui.md` cita o relatório de `fala-research` só pelo nome (sem caminho local), como já faz o `checks.md`.
- Lacuna de precisão menor, não bloqueante (C17): a cláusula "nenhum arquivo de `docs/decisions/` passa a ter `status: accepted`" não é afirmada pelo T1, que só olha a 0017. Quem a prova é o `git diff`, aqui e indiretamente pela lista de arquivos de C19. Hoje está provada. Numa rodada futura, o T1 não a pegaria sozinho.
- Lacuna de precisão menor, não bloqueante (C18): a rampa é afirmada por substring `| 12/16 |` etc., sem o peso, e os raios por frase em prosa. Isso basta para o que o check pede (tamanhos, 4 e 8 px), mas um peso trocado na tabela passaria.

## Gate

`bun src/styles/tokens.test.ts` - 15 checks impressos (C1–C13, C17, C18), 0 falhas, exit 0
