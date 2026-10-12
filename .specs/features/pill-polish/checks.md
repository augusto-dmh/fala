# pill-polish checks

Profile: light
Plan: `.specs/features/pill-polish/plan.md`

6 checks in 1 slice · 0 one-way doors · 0 open

Cargo com `CARGO_TARGET_DIR=C:\f\pill-polish`, na raiz do worktree.

## Checks

**C1** - A falha de cola do ditado mantém o texto, emite `paste-error` (toast da janela principal)
e mostra `paste_failed` em vez de esconder; o hide roda sem falha ou quando a pílula não pode
tomar o overlay. A do colar o último também mostra a pílula, e com a pílula no ar o colar o último
cola o texto que falhou.
Proof: `cargo test -p fala --lib actions::tests::failed_paste_keeps_the_pill_up_to_say_so` e
`last_dictation::tests::failed_paste_is_kept_until_the_pill_ends`

**C2** - `paste_failed` mede como um aviso, é estado ativo, aparece sem consultar `overlay_style`,
nunca sobre uma gravação nem sobre a reunião (ADR-0005), guarda o texto e manda a geração à página;
o fim (pela geração, na thread principal) esquece o texto e só esconde a própria pílula; outro
estado esquece o texto; guarda de 60 s.
Proof: `cargo test -p fala --lib overlay::tests::paste_failed_pill`

**C3** - `pillModel`: atalho do `paste_last` (ou `null`), 5 s sem hover e nada com hover, a segunda
linha (atalho, só clique, "Copiado"); a pílula renderiza título e dica, `role="button"`, ponto
`warn` (ADR-0017), o clique inteiro chama a cópia; strings pt e en.
Proof: `bun src/overlay/pill.test.tsx` (pill-polish C3)

**C4** - O overlay lê o atalho das settings sem espera nova, para o timer com o mouse no cartão,
reinicia o timer a cada falha (`paste-failed`), encerra por `endPasteFailed(geração)`, copia por
`copyFailedPaste` e mostra "Copiado"; o toast de `paste-error` segue.
Proof: `bun src/overlay/pill.test.tsx` (pill-polish C4 e upstream-catchup PR2 C7)

**C5** - O clique na pílula de reunião chama `openMeetingFromPill`; o comando abre a janela e emite
`open-meeting` com o id; o app vai para Reuniões e a página abre a sessão (montando depois ou já
aberta).
Proof: `bun src/overlay/pill.test.tsx` (pill-polish C5) e
`cargo test -p fala --lib overlay::tests::meeting_pill_opens_its_session`

**C6** - lint, formato, tipos, traduções e clippy limpos.
Proof: `bun run lint`, `bun run format:check`, `bunx tsc --noEmit`, `bun run check:translations`,
`cargo clippy --workspace --all-targets -- -D warnings`
