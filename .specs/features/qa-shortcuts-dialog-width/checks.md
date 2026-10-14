# qa-shortcuts-dialog-width — checks

R3, R9, R10 e o resto do Q52 da segunda passada de QA (`research/27-reqa-fala.md`).

| ID | Check | Proof |
| --- | --- | --- |
| W1 | o `Dialog` ganha `size` (`md` 32rem, `lg` 42rem) e o diálogo de atalhos usa `lg`: o `max-w-2xl` em `className` perdia para o `max-w-lg` padrão; agora um `max-w-*` em `className` substitui o tamanho | `bun src/components/settings/shortcutsWidth.test.tsx` W1 |
| W2 | os chips de tecla do diálogo são sans, como no onboarding e na pílula; todo `kbd` herda a fonte do texto | `shortcutsWidth.test.tsx` W2 |
| W3 | "Comportamento do atalho" e o título da linha falam do atalho de ditar ("Atalho de ditar"); o campo do atalho (onboarding e Configurações) não quebra linha | `shortcutsWidth.test.tsx` W3 |
| W4 | portões | `bun run lint`, `bunx prettier --check .`, `bunx tsc --noEmit`, `bun run check:translations`, os testes de settings, ui, onboarding e shell |
