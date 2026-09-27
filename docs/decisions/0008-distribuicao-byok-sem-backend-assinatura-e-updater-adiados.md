---
status: accepted
date: 2026-09-26
---
# Distribuição: BYOK com chaves no keyring, sem backend; assinatura de código e auto-update adiados

## Contexto e problema
Colegas vão instalar. Os serviços de nuvem (Gemini, ElevenLabs, Claude) custam por uso. Embutir chaves no binário viola a política de sigilo de credenciais e faz o autor pagar por todos. Assinatura de código no Windows custa e o SmartScreen não tem mais bypass por certificado EV desde 2024. A chave do updater do Tauri (minisign) é irrecuperável se perdida: perder a chave é perder a capacidade de atualizar todas as instalações.

## Opções consideradas
* BYOK: cada usuário coloca as próprias chaves, guardadas no keyring do SO; sem chave, o app funciona só local.
* Chaves do autor embutidas no build interno.
* Backend proxy com conta.

## Decisão
BYOK. Sem backend. Instalador sem assinatura por enquanto, com o aviso do SmartScreen documentado no README. Updater desligado até o primeiro release; quando ligado, a chave privada é gerada uma vez, guardada no gerenciador de senhas do autor e nunca no repositório.

### Consequências
* Bom: zero custo e zero dado de colegas passando pelo autor; sem credencial em binário.
* Ruim: onboarding com tela de chaves; aviso do SmartScreen na instalação; sem update automático até decidir.
* Obrigatório: `cargo deny` e um teste de que nenhum arquivo versionado contém padrão de chave; `docs/RELEASE.md` antes do primeiro `.exe`.

## Confirmação
CI roda scanner de segredos; `Settings` não tem campo de chave (só referência ao keyring).
