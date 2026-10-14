# qa-ai-needs-key — checks

Achados Q37 e Q42 (chips de "Apps sem IA") do QA de 2026-10-13. Diff de uma frase; sem plano.

| ID | Achado | Check | Proof |
| --- | --- | --- | --- |
| K1 | Q37 | ligada sem a chave do Gemini, a IA "aguarda a chave" num estado só: a linha diz que nada é formatado nem enviado até salvar a chave e a Privacidade mostra "Aguardando a chave"; o interruptor continua livre (dá para desligar antes de salvar a chave) e o campo da chave nunca trava; sem as configurações, a Privacidade segue dizendo "ligado" | `bun src/components/settings/aiNeedsKey.test.tsx` K1, `privacy.test.tsx` C2 |
| K2 | Q42 | os chips de "Apps sem IA" ficam dentro do card do campo | `aiNeedsKey.test.tsx` K2 |
| K3 | — | portões | `bun run lint`, `bunx prettier --check .`, `bunx tsc --noEmit`, `bun run check:translations`, os testes de `src/components/settings/` |
