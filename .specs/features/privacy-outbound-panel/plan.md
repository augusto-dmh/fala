# privacy-outbound-panel — "O que sai deste computador", por canal

## Problem

As saídas de dados do Fala estão espalhadas por ADRs (0004/0012 ditado → Gemini; 0005 reunião →
ElevenLabs; 0016 notas → Anthropic; 0010 MCP local) e por toggles em telas diferentes. Quem
instala não tem onde conferir o que sai, para onde e se está ligado agora. A privacidade é a
reclamação nº 1 contra o Wispr, que não mostra o que envia.

Quando isto entrar, Configurações → Sobre tem uma seção "Privacidade", só leitura: uma linha por
canal com o que sai, para onde e o estado atual; onde ficam as chaves; onde fica o áudio de
reunião (com botão para abrir); e um link para as ADRs no repositório público. Nenhum valor de
chave aparece.

## Flow

Sem Rust. Tudo lido do que o front já recebe.

1. `PrivacySection` (novo) - lê `useSettings().settings`, `commands.meetingKeys()` (só booleanos),
   `mcp.toml` em `$APPDATA` via `@tauri-apps/plugin-fs` (escopo já liberado em
   `capabilities/default.json`) e `commands.getAppDirPath()`
2. `privacyInputs(settings, meetingKeys, mcpToml)` (novo, puro) - reduz tudo a booleanos e ao
   nome do provedor; o valor das chaves para aqui
3. `privacyRows(inputs)` (novo, puro) - as linhas: ditado, pós-processamento experimental (só
   quando ligado), transcrição de reunião, notas de reunião, MCP, telemetria/conta/servidor
4. `PrivacyView` (novo, apresentação) - renderiza linhas e as notas de chaves, áudio e ADRs
5. `AboutSettings` (existe) - ganha `<PrivacySection />` depois do grupo "Sobre"

## Impact

| Front | What changes |
| --- | --- |
| ui | seção nova em Sobre; chaves novas em `settings.about.privacy` de `pt` e `en` |
| behaviour | nenhum comportamento muda; a tela só lê. Abrir a pasta de áudio usa `revealItemInDir` (permitido por `opener:default`) e cai em `openAppDataDir` se a pasta ainda não existe |
| domain | termo novo: canal de saída. O estado do MCP segue a regra do `crates/mcp/src/config.rs` (só `enabled = true` liga), com o desvio seguro: na dúvida a tela diz "ligado" |

## Decisões (recomendação da definição, seguida)

- Seção dentro de Sobre, não aba própria (menos diff em arquivos que outros painéis tocam).
- O pós-processamento herdado do Handy (Avançado → experimental, `post_process_enabled`) é uma
  saída real de texto ditado para o provedor escolhido. A definição não o lista; a tela o mostra
  como linha extra **só quando ligado**, para a lista ser exata sem expor um recurso escondido.
- "Só local" (ADR-0005/0016) ainda não tem UI (`local_only` é sempre `false`); a tela diz "só
  quando você pede a transcrição/as notas", que é o comportamento atual, sem citar a opção.
- A ADR-0010 é proposta; o MCP é `fala-cli mcp`, só com `mcp.toml`. A tela lê o arquivo da
  pasta padrão (`$APPDATA/br.com.augusto.fala`), a mesma do `fala-mcp`.
- Quem mudar uma dessas saídas muda esta tela no mesmo PR (registrado no `checks.md`).

## Landing

Nada é porta de uma via: a seção é só leitura e as chaves de i18n são lidas só por este app.

## Criteria

1. A seção existe em Sobre com as linhas ditado, transcrição de reunião, notas de reunião, MCP e
   telemetria, nesta ordem, nos dois idiomas.
2. Ditado: estado reflete `llm_enabled` (ligado/desligado) e a presença da chave `gemini`
   (vazia ou só espaços = sem chave).
3. Reunião: estado reflete `meeting_keys` (ElevenLabs e Anthropic, presente/ausente; "não
   verificado" se o comando falhar).
4. MCP: ligado só com `enabled = true` no `mcp.toml`; arquivo ausente = desligado.
5. Telemetria: "nada", "não existe".
6. Pós-processamento experimental: linha só aparece com `post_process_enabled`, com o nome do
   provedor.
7. Nenhum valor de chave aparece no markup, mesmo com chaves presentes.
8. Notas: chaves no cofre do sistema (Gerenciador de Credenciais no Windows); caminho da pasta de
   áudio de reunião com botão Abrir; link para `docs/decisions` no GitHub.
