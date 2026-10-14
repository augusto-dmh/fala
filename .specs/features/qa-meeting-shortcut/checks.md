# qa-meeting-shortcut — checks

R1 e R11 da segunda passada de QA (`research/27-reqa-fala.md`). Decisão do orquestrador: "Gravar ou
parar reunião" recusa, não só avisa, qualquer combinação que sobreponha outro atalho do Fala, nos
dois sentidos, e recusa combinação só de modificadores (ADR-0005). As outras ações seguem como
estão. O diálogo mostra o problema ao abrir.

| ID | Check | Proof |
| --- | --- | --- |
| C1 | `conflict::check` recusa `meeting_toggle` só com modificadores (`MeetingNeedsKey`) e a sobreposição quando um dos lados é a reunião (`MeetingOverlap`), dos dois lados, antes de qualquer aviso; as outras sobreposições seguem como aviso; o reset recusa as regras da reunião | `cargo test -p fala --lib shortcut` (`conflict_table`, `meeting_overlap_is_refused_from_either_side`) |
| C2 | uma combinação de reunião salva antes das regras (só modificadores, ou sobreposta) não é registrada: `register_shortcut` passa por `conflict::registrable` | `cargo test -p fala --lib shortcut` (`saved_meeting_combinations_that_break_the_rules_are_not_registered`) |
| C3 | o diálogo pede ao backend (`binding_problems`, as mesmas regras do save) o que os atalhos salvos já têm de errado, ao abrir e quando eles mudam; uma regra da reunião vence um aviso; um save limpa as notas velhas das outras linhas | `bun src/components/settings/meetingShortcut.test.ts` S1, S2 |
| C4 | as recusas novas têm texto em pt e en; `bindings.ts` regerado | `meetingShortcut.test.ts` S2 |
| C5 | portões | `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p fala --lib shortcut`, `bun run lint`, `bunx prettier --check .`, `bunx tsc --noEmit`, `bun run check:translations`, os testes de `src/components/settings/` |
