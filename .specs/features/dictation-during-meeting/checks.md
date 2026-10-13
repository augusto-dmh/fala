# dictation-during-meeting checks

Profile: light
Plan: `.specs/features/dictation-during-meeting/plan.md`

Cargo com `CARGO_TARGET_DIR=C:\f\dictation-during-meeting`, na raiz do worktree.

## Checks

**C1** - `MicHub`: dois taps do mesmo dispositivo abrem um stream só (contador = 1), cada um no seu
canal, sem buraco; o último tap fecha o stream; dispositivos diferentes têm um stream cada; um feed
com falha não é reaproveitado; erro de abertura chega a quem pediu; tap não drenado descarta e conta
(AC 1) — PR 1
Proof: `cargo test -p fala-audio hub::`

**C2** - `Mic` guarda quadros inteiros e reduz ao canal ou à média (AC 1) — PR 1
Proof: `cargo test -p fala-audio mic::`

**C3** - `MeetingRecorder`: zeros exatos do pré-buffer ao fim do ditado e nada fora dele; o canal do
sistema não muda; crash no meio do ditado só deixa zeros no disco; ditado aberto termina com a
gravação; dois ditados sobrepostos viram um trecho; lookback além do hold conta `omit_late`; mic a
16 kHz zera o miolo e o trecho bate com ±10 ms (AC 2, 3) — PR 2
Proof: `cargo test -p fala-audio meeting::recorder::`

**C4** - Desktop: ditado e gravador pelo mesmo `MicHub`; início do ditado manda `BeginOmit`, fim,
cancelamento e descarte mandam `EndOmit`; recusa só no presencial (AC 4) — PR 3
Proof: `cargo test -p fala --lib meeting`

**C5** - `MuteWatch` e o contador de silêncio da sessão não contam o mic de um período com ditado;
um ditado de 3 min numa call em que só a pessoa fala não arma "Ainda está gravando?" (AC 5) — PR 3
Proof: `cargo test -p fala-meeting`, `cargo test -p fala --lib meeting`

**C6** - O trecho omitido vira linha marcadora na transcrição, no SQLite, no `.md` e no payload das
notas (AC 6) — PR 3
Proof: `cargo test -p fala-storage`, `cargo test -p fala-notes`, `cargo test -p fala --lib meeting`

**C7** - Portões: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D
warnings`, testes dos crates tocados, `bun run lint`, `bun run format:check`, `bunx tsc --noEmit`,
`bun run check:translations`, `scripts/check-brand.sh`
Proof: os comandos acima, exit 0

## Manual on Windows (fora da tabela)

M1 gravar uma call real e ditar no chat do Teams no meio (um curto, um longo de ~1 min) · M2 o
marcador aparece na transcrição e o trecho toca silêncio · M3 a pílula avisa enquanto dita · M4 no
modo presencial o ditado é recusado com a mensagem.

## Coverage

| Set (size) | Member -> proof | Unproven |
| --- | --- | --- |
| fins de ditado (4) | soltar · cancelar · descartar · fim da gravação: C3, C4 | M1 |
| dispositivos (2) | mesmo · diferente: C1, C3 | M1 |
| canais do WAV (2) | mic zerado · sistema intacto: C3 | — |

Swept: concurrency C1 (trava do hub durante a abertura) · crash C3 · o resto n/a.
