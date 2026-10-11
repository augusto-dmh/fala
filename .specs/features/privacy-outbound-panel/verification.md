# privacy-outbound-panel verification

Checks: `.specs/features/privacy-outbound-panel/checks.md` · commit `2d394ce` · Windows 11
(Alienware), 2026-10-11. Cada prova rodou de novo, do zero, depois do commit.

Verdict: **PASS** para C1-C11. A prova manual na tela fica como roteiro no PR (não é check).

| Check | Proof | Result |
| --- | --- | --- |
| C1 linhas e ordem | `bun src/components/settings/about/privacy.test.tsx` | `C1 ok` |
| C2 ditado (6 casos) | idem | `C2 ok` |
| C3 reunião (5 casos) | idem | `C3 ok` |
| C4 MCP (11 casos) | idem | `C4 ok` |
| C5 telemetria | idem | `C5 ok` |
| C6 pós-processamento herdado | idem | `C6 ok` |
| C7 nenhuma chave no markup | idem | `C7 ok` |
| C8 pt e en | idem + `bun run check:translations` | `C8 ok`; exit 0 |
| C9 chaves, áudio, ADRs | idem | `C9 ok` |
| C10 em Sobre | idem | `C10 ok` |
| C11 front | `bun run lint` · `bunx prettier --check .` · `bunx tsc --noEmit` · `bun run build` | exit 0 nos quatro |

O teste saiu com exit 0 e imprimiu `C1 ok` a `C10 ok`. `scripts/check-brand.sh` também saiu 0.

## A prova pega o que diz pegar

- Mutação: com `geminiKey: true` fixo em `privacyInputs`, o teste parou em C2 com
  `AssertionError`; com o arquivo restaurado, voltou a passar até C10.
- C7 usa sentinelas de chave em `gemini` e `openai` e renderiza nos dois idiomas; C8 renderiza
  com um i18next de verdade e falha com chave crua ou `{{` sobrando.
- C9 lê `apps/desktop/src/meeting/mod.rs` e falha se o backend deixar de guardar o áudio de
  reunião em `<pasta de dados>/audio`.

## Fora da prova

- A seção na tela (layout, os dois idiomas, os botões Abrir e Ver no GitHub, a leitura real de
  `mcp.toml` e de `meeting_keys`): exige o app aberto; roteiro no PR.
- `src/styles/tokens.test.ts` não roda no Windows (`new URL(..).pathname` vira `/C:/...` e o
  `readdirSync` falha com `ENOENT`), num arquivo que esta mudança não toca; no lugar dele, um grep confirmou que os arquivos novos não têm hex nem
  aliases herdados (`logo-primary`, `mid-gray`, `background-ui`).
