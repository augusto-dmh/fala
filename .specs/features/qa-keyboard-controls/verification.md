# qa-keyboard-controls — verification

Máquina: Windows 11 (Alienware 16), 2026-10-13. Verificado pelo próprio executor.

## Verdict

**PASS** em K1–K5. O app rodando não foi aberto (ver "Não provado").

| ID | Result | Evidence |
| --- | --- | --- |
| K1 | PASS | K1 ok (pula desabilitada, volta nas duas pontas, Home/End, tecla qualquer e lista sem habilitadas dão `null`) |
| K2 | PASS | K2 ok (markup numa `SettingContainer`: `aria-labelledby` com o id do título e o do valor, `aria-haspopup`, `aria-expanded="false"`, sem `aria-controls`; fora dela, sem `aria-labelledby`; fonte: papéis, Esc com `stopPropagation` e `close(true)`, Tab nos dois lugares, Enter/Espaço por `event.detail === 0`, foco por valor com `options` no efeito) |
| K3 | PASS | K3 ok |
| K4 | PASS | K4 ok |
| K5 | PASS | lint sem aviso; prettier limpo; tsc sem erro; traduções completas; `controls.test.tsx` (C1–C24, com o regex do título aceitando o `id`) e os testes de settings, Reuniões, onboarding e shell passam |

## Não provado

Roteiro no app: em Configurações › Geral, Tab até o seletor de microfone, ↓ abre a lista com o
foco na opção atual, ↓/↑ andam, Enter escolhe e o foco volta ao botão; abrir de novo e Esc fecha;
um leitor de tela lê "Restaurar o padrão" no ↻ e o título em cada toggle.
