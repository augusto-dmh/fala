# qa-pill-new-meeting — verification

Máquina: Windows 11 (Alienware 16), 2026-10-13. Verificado pelo próprio executor.

**PASS** em M1–M4: `pillMeeting.test.tsx` M1–M3 ok; `cargo test -p fala --lib overlay` 22 passed
(com `idle_dimensions` em 300×96); `cargo test -p fala --lib tray` 16 passed; os testes de
`src/overlay/` passam; lint sem aviso, prettier limpo, tsc sem erro, traduções completas, `cargo
fmt` limpo; `bindings.ts` regerado pelo `fala.exe --list-devices` (só o comando novo).

Não provado no app: passar o mouse na pílula em repouso e clicar "Nova reunião": sem o
consentimento aceito, abre a janela no aviso; com ele, a reunião começa e a pílula de reunião
aparece; clique direito → "Gravar reunião" faz o mesmo.
