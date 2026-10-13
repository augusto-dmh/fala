# qa-meeting-keys — verification

Máquina: Windows 11 (Alienware 16), 2026-10-13. Verificado pelo próprio executor.

## Verdict

**PASS** em K1–K6. O app rodando não foi aberto (ver "Não provado").

| ID | Result | Evidence |
| --- | --- | --- |
| K1 | PASS | K1 ok (ElevenLabs no transcrever, Anthropic nas notas, `null` sem passo ou para outro erro; botão `meeting.keys.add`) |
| K2 | PASS | K2 ok (efeito por `selectedId` e `fail` só com a sessão ainda aberta) |
| K3 | PASS | K3 ok (dicas com e sem chave, Granola sem dica de transcrever, `null` antes das chaves; barra renderizada com `disabled`, `title` e o texto no meio); `notesPolish.test.tsx` com o `notesOff` comum aos três botões |
| K4 | PASS | K4 ok; `home.test.tsx` com `w-full space-y-4` e sem `max-w-2xl` |
| K5 | PASS | K5 ok; `import.test.tsx` passa |
| K6 | PASS | lint sem aviso; prettier limpo; tsc sem erro; traduções completas; os 17 testes de Reuniões e `shell.test.tsx` passam |

## Não provado

Roteiro no app (cópia portátil, sem chaves): abrir uma reunião importada de um WAV: "Transcrever"
e "Gerar notas" desligados, com a dica de cada chave; voltar à lista e abrir outra: nenhum erro
velho; importar o CSV do Granola: a prévia aparece num card abaixo da busca; abrir "Importar" a
900 px: URL numa linha e o ✕ no canto.
