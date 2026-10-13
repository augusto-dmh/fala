# qa-meeting-keys — checks

Achados Q16, Q17, Q21, Q22 e a largura de Reuniões do Q54, do QA de 2026-10-13. O backend não diz
qual chave falta (`missing_key` não tem campo, e o pipeline está com outro painel): a página sabe
pelo passo que falhou (transcrever → ElevenLabs, notas → Anthropic). Diff de uma frase; sem plano.

| ID | Achado | Check | Proof |
| --- | --- | --- | --- |
| K1 | Q16 | o erro de chave diz qual falta e oferece [Adicionar chave] (abre Configurações); os outros erros seguem iguais | `bun src/components/meeting/meetingKeys.test.tsx` K1 |
| K2 | Q16 | o erro some ao trocar de reunião ou voltar à lista; o de uma execução que termina depois que a pessoa saiu não aparece; erro e passo são um estado só | `meetingKeys.test.tsx` K2 |
| K3 | Q17 | sem a chave, "Transcrever" e "Gerar notas" ficam desligados com o motivo na dica e em texto no meio da barra; "Gerar notas", o "gerar de novo" e o "Tentar de novo" pedem transcrição ou notas digitadas (salvas ou não); sem áudio (Granola) não há dica de chave para transcrever; antes de saber das chaves nada desliga | `meetingKeys.test.tsx` K3 e `notesPolish.test.tsx` |
| K4 | Q21, Q54 | a prévia do Granola vem num card depois das ações (lista) ou dos avisos (reunião); Reuniões tem a largura das outras páginas | `meetingKeys.test.tsx` K4 e `home.test.tsx` |
| K5 | Q22 | o painel de importar põe a URL numa linha própria (campo com rótulo) e o Fechar num ✕ no canto | `meetingKeys.test.tsx` K5 e `import.test.tsx` |
| K6 | — | portões | `bun run lint`, `bunx prettier --check .`, `bunx tsc --noEmit`, `bun run check:translations`, os testes de `src/components/meeting/` e `shell.test.tsx` |
