# qa-meeting-shortcut — verification

Máquina: Windows 11 (Alienware 16), 2026-10-14. Verificado pelo próprio executor.

**PASS** em C1–C5: `cargo test -p fala --lib shortcut` 27 passed; `meetingShortcut.test.ts` S1 e S2
ok; os testes de `src/components/settings/` passam; clippy sem aviso; lint, prettier, tsc e
traduções limpos; `bindings.ts` regerado (os dois tipos novos de recusa, `BindingProblem` e
`bindingProblems`).

Não provado no app: no diálogo de atalhos, gravar Ctrl+Shift em "Gravar ou parar reunião" mostra a
recusa; com um atalho de reunião salvo antes como Ctrl+Shift, o diálogo já abre com o alerta e
segurar Ctrl+Shift+Space dita sem gravar reunião.
