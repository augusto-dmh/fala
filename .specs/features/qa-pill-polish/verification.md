# qa-pill-polish — verification

Máquina: Windows 11 (Alienware 16), 2026-10-13. Verificado pelo próprio executor.

## Verdict

**PASS** em P1–P4. O app rodando não foi aberto (ver "Não provado").

| ID | Result | Evidence |
| --- | --- | --- |
| P1 | PASS | P1 ok (nome lido antes do stop e passado ao `show_notice`; pergunta antes do nome em pt e en); `notice.test.tsx` passa com `noticeTextKey` |
| P2 | PASS | P2 ok; `pill.test.tsx` (pill-idle C6) com as cores novas |
| P3 | PASS | `cargo test -p fala --lib tray`: 16 passed |
| P4 | PASS | lint sem aviso; prettier limpo; tsc sem erro; traduções completas; os testes de `src/overlay/` passam; clippy e testes do workspace no pre-push |

## Não provado

Roteiro no app: ligar a pílula em repouso e vê-la cinza, um pouco acima da barra de tarefas;
ditar em silêncio e ler "Microphone muted? No sound from <mic>.";
clique direito na pílula: "Copy last dictation" copia o último ditado.
