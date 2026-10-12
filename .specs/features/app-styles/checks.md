# app-styles checks

Profile: light · Plan: `.specs/features/app-styles/plan.md`

Cargo com `CARGO_TARGET_DIR=C:\f\app-styles`, na raiz do worktree.

**C1** - Servidor falso: com estilo para `SLACK`, o corpo para `slack` é o de antes com só o
`systemInstruction` acrescido de `\n\n{STYLE_INSTRUCTION}\n{instrução}` no fim, depois do
dicionário; app sem estilo, instrução em branco e ditado sem app mandam o corpo de antes, byte a
byte (ADR-0004/0012)
Proof: `cargo test -p fala-postproc --test fake_gemini app_style_is_the_only_payload_change`

**C2** - No desktop, `chrome → email` manda a instrução do E-mail no fim do prompt, `notepad`
manda o prompt sem ela, e `1password` (LLM desligado) não manda nada
Proof: `cargo test -p fala --lib llm_auto::tests::app_style_ends_the_llm_prompt`

**C3** - Quatro estilos prontos (`message, email, document, code`) com nome e instrução; store
vazio carrega os quatro e nenhum mapeamento
Proof: `cargo test -p fala --lib app_styles::tests::four_ready_styles_with_portuguese_instructions`

**C4** - Normalização: nome e instrução aparados, estilo sem nome cai, id vazio ou repetido
vira `style-N`; mapeamento pelo nome do executável, um por app (o último vale, no lugar do
primeiro), estilo inexistente cai; `instructions` ignora estilo apagado
Proof: `cargo test -p fala --lib app_styles::tests`

**C5** - `newStyleId` não repete id; `setAppStyle` mapeia o app aparado e em minúsculas e
troca o estilo de um app já mapeado no lugar
Proof: `bun src/components/settings/appStyles.test.tsx` (linha `C5 ok`)

**C6** - `AppStyles` mostra cada estilo com nome e instrução e cada mapeamento como
`app → nome do estilo`; está na tela de IA
Proof: `bun src/components/settings/appStyles.test.tsx` (linha `C6 ok`)

**C7** - As chaves novas existem em pt e en, nenhuma vazia
Proof: `bun src/components/settings/appStyles.test.tsx` (linha `C7 ok`); `bun run check:translations`
