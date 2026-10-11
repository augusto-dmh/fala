# privacy-outbound-panel checks

Profile: light
Plan: `.specs/features/privacy-outbound-panel/plan.md`

11 checks in 1 slice · 0 one-way doors · 0 open. Sem Rust: nenhum `cargo` compila o que muda.
Todos os `bun` rodam na raiz do worktree.

## Checks

### S1 - a seção Privacidade · 8 files · ~12k

**C1** - Sem nada experimental ligado (e com `settings` nulo), as linhas são, nesta ordem: ditado, transcrição de reunião, notas de reunião, MCP, telemetria (AC 1)
Proof: `bun src/components/settings/about/privacy.test.tsx` (linha `C1 ok`)

**C2** - Ditado: `llm_enabled` ligado/desligado × chave `gemini` presente/ausente/só espaços; sem `llm_enabled` vale o padrão do backend (ligado); chave de outro provedor não conta (AC 2, tabela com 6)
Proof: `bun src/components/settings/about/privacy.test.tsx` (`C2 ok`)

**C3** - Reunião: as 4 combinações de `meeting_keys` e `null` ("não verificado") nas linhas de transcrição e notas (AC 3, tabela com 5)
Proof: `bun src/components/settings/about/privacy.test.tsx` (`C3 ok`)

**C4** - MCP: ligado só com `enabled = true`; ausente, vazio, `false`, `"true"`, comentado e `truex` = desligado; quando a leitura da tela diverge da do `crates/mcp` (sob tabela, chave entre aspas), ela diz "ligado" (AC 4, tabela com 11)
Proof: `bun src/components/settings/about/privacy.test.tsx` (`C4 ok`)

**C5** - Telemetria, conta, servidor: "não existe" (AC 5)
Proof: `bun src/components/settings/about/privacy.test.tsx` (`C5 ok`)

**C6** - O pós-processamento herdado aparece só com `post_process_enabled`, logo depois do ditado, com o `label` do provedor (ou o id, sem label) no destino (AC 6)
Proof: `bun src/components/settings/about/privacy.test.tsx` (`C6 ok`)

**C7** - Com chaves sentinela em `gemini` e `openai` e as duas chaves de reunião presentes, nenhum sentinela aparece no markup em pt nem em en; `PrivacyView.tsx` e `PrivacySection.tsx` não citam `api_keys` (AC 7)
Proof: `bun src/components/settings/about/privacy.test.tsx` (`C7 ok`)

**C8** - Em pt e em en, cada linha (as seis), cada estado, títulos, colunas e notas existem no locale, aparecem no markup, e não sobra chave crua nem `{{` (AC 1, AC 8)
Proof: `bun src/components/settings/about/privacy.test.tsx` (`C8 ok`)
Proof: `bun run check:translations`

**C9** - A pasta de áudio é `<pasta de dados>/audio` (Windows e Linux; o backend usa `data.join("audio")`), aparece com o botão Abrir, que revela a pasta e cai em `openAppDataDir`; as chaves "no Gerenciador de Credenciais"; o link vai a `github.com/augusto-dmh/fala/tree/main/docs/decisions`; o estado vem de `meetingKeys()` e de `mcp.toml` em `AppData` (AC 8)
Proof: `bun src/components/settings/about/privacy.test.tsx` (`C9 ok`)

**C10** - `AboutSettings` renderiza `<PrivacySection />` logo depois do grupo "Sobre" (AC 1)
Proof: `bun src/components/settings/about/privacy.test.tsx` (`C10 ok`)

**C11** - O front continua limpo e compila
Proof: `bun run lint` · `bunx prettier --check .` · `bunx tsc --noEmit` · `bun run build`

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| canais (6) | ditado C1 · experimental C6 · transcrição C3 · notas C3 · MCP C4 · telemetria C5 | - |
| estados (6) | on · off · keySet · keyMissing · unknown · doesNotExist, C8 renderiza todos | - |
| locales (2) | pt C8 · en C8 | - |
| fontes do estado (3) | settings C2/C6 · `meeting_keys` C3 · `mcp.toml` C4 | - |
| ações (2) | abrir pasta de áudio C9 (estrutural) · abrir ADRs C9 (estrutural) | clique real: manual |

- Manual (precisa de tela, não roda aqui): abrir Configurações → Sobre no `bun run tauri dev`, conferir a seção nos dois idiomas, clicar em Abrir (com e sem reunião gravada) e em Ver no GitHub. Fica como roteiro no PR, fora da tabela.
- Quem mudar uma saída das ADRs 0004/0012, 0005, 0008, 0010, 0011 ou 0016 muda `privacyModel.ts`, os locales e este teste no mesmo PR.

## Swept

- validation: C4 (TOML do MCP), C2 (chave em branco)
- failure modes: `meeting_keys` falha -> "não verificado" C3; `mcp.toml` ilegível -> desligado C4; pasta de áudio inexistente -> abre a pasta de dados C9
- idempotency: n/a - só leitura
- authorization: n/a - tela local; nenhuma permissão nova (fs em `$APPDATA` e `revealItemInDir` já estavam liberados)
- concurrency: n/a
- data lifecycle: n/a - nada é gravado
- dependency failure: C3, C4
- state transitions: n/a
- observability: n/a - nenhum log com conteúdo; falhas de comando vão a `console.error` sem valores

## Handoff

- S1 = ~12k, um builder.
- **Settled mid-build:** linha extra para o pós-processamento herdado (Avançado → experimental), só quando ligado; a opção "só local" das ADRs 0005/0016 ainda não tem UI (`local_only` sempre `false`), então a tela diz "só quando você pede".
- **Abandoned:** nada.
- **Boundary:** C1-C11 em `feat/privacy-outbound-panel`; a prova manual na tela fica no PR.
