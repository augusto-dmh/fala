# qa-shortcuts-dialog-width — verification

Máquina: Windows 11 (Alienware 16), 2026-10-13. Verificado pelo próprio executor.

**PASS** em W1–W4: `shortcutsWidth.test.tsx` W1–W3 ok; os testes de settings, ui, onboarding e
shell passam; lint, prettier, tsc e traduções limpos.

Não provado no app: abrir Configurações › Atalhos a 915 px e medir o diálogo (~672 px); as teclas em
sans; no passo 2 do onboarding, "Ctrl + Shift + Space" numa linha.
