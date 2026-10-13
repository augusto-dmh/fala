# qa-ai-needs-key — verification

Máquina: Windows 11 (Alienware 16), 2026-10-13. Verificado pelo próprio executor.

**PASS** em K1–K3: `aiNeedsKey.test.tsx` K1 e K2 ok; `privacy.test.tsx` C2 com "waitingForKey"
nos casos ligados sem chave e um caso novo do padrão ligado com chave; `cleanupLevel.test.tsx` com
`dimmed={!formatting}`; lint sem aviso, prettier limpo, tsc sem erro, traduções completas.

Não provado no app: sem a chave do Gemini, a linha diz "Aguardando a chave do Gemini abaixo" e a
Privacidade "Aguardando a chave"; desligar antes de salvar e salvar a chave mantém desligado; os
apps sem IA aparecem no mesmo card.
