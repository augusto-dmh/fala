# meeting-people-series checks

Profile: light · Windows 11, `CARGO_TARGET_DIR=C:\f\meeting-people-series`

## PR 1 — Pessoas

- **C1** chave (caixa, acento, NFD, espaços, pontas, e-mail). `cargo test -p fala-storage --lib meeting_directory`
- **C2** uma vez por reunião, grafia mais recente, contagem, ordem, round-trip com `reindex`;
  lixeira e rascunho vazio fora. `cargo test -p fala-storage --test meeting_directory`
- **C3** comandos e `bindings.ts` (`fala.exe --list-devices`).
- **C4** alternador, tabela, vazio, filtro, uma pessoa. `bun src/components/meeting/people.test.tsx` (P1-P4)

## PR 2 — Juntar e sugerir

- **C5** juntar/renomear reescreve participantes (sem repetir a pessoa) e falantes de toda
  reunião, e os `.md`; mesma grafia ou chave sem ninguém não muda nada.
  `cargo test -p fala-storage --test meeting_directory merging`
- **C6** "⋯" com "Juntar com…"/"Renomear"; confirmação com as duas grafias e a contagem;
  sugestão por prefixo sem acento, até 5, sem repetidos; usada em "Quem estava" e "Pessoa N".
  `bun src/components/meeting/people.test.tsx` (P5-P8)
