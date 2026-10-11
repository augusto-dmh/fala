---
status: proposed
date: 2026-10-11
---
# O updater liga antes do primeiro release, com feed no GitHub Release e instalação só fora de captura

## Contexto e problema
A ADR-0008 deixou o updater desligado "até o primeiro release". Esse release está perto: os
colegas vão receber o primeiro `.exe`. Uma instalação sem updater nunca se atualiza sozinha.
Quem instalar uma versão sem ele vai reinstalar à mão a cada versão, para sempre. O lado cliente
do updater do Tauri veio do Handy e já está no código (plugin, capabilities, item de bandeja,
`UpdateChecker`, "What's New", template NSIS com modo passivo), desligado por `UPDATER_ENABLED`
e travado por `scripts/ci/test_release_config.py`.

No Windows, instalar um update significa fechar o app: o plugin roda o NSIS e chama `exit(0)`.
O Fala grava reuniões e dita. Um update no meio de uma captura perde áudio. Wispr Flow e Granola
resolvem isso com um portão: só instalam fora de captura (pesquisa em
`fala-research/research/22-atualizacao-e-releases.md`).

A chave minisign do updater é irrecuperável. Perdê-la é perder o update de todas as instalações.
A ADR-0008 manda guardá-la no gerenciador de senhas e nunca no repositório.

## Opções consideradas
* Manter o updater desligado e distribuir por reinstalação manual.
* Ligar com feed estático no GitHub Release, build local e instalação por clique ou ao sair.
* Ligar com feed no GitHub Pages, build no GitHub Actions e instalação automática quando ocioso.
* Ligar com servidor de update próprio ou CrabNebula Cloud.

## Decisão
Ligar o updater antes da `v0.1.0`, com feed estático no GitHub Release, build local e instalação
por clique ou ao sair, nunca durante captura. É o menor passo que põe todo colega no ciclo de
update desde o primeiro `.exe`, sem backend e sem a chave sair da máquina do mantenedor. Esta
ADR substitui a 0008 **só no ponto do updater**. BYOK, chaves de API no keyring, ausência de
backend e instalador sem assinatura de código continuam valendo pela 0008.

1. **Feed:** `https://github.com/augusto-dmh/fala/releases/latest/download/latest.json`, anexado a
   cada release. O alias `latest` nunca serve draft nem prerelease, então um release só é oferecido
   depois de publicado. Pré-releases saem como `vX.Y.Z-rc.N` marcados como prerelease, sem canal
   beta no app. As releases imutáveis do GitHub ficam desligadas por enquanto, para manter a saída
   de voltar um release ruim para rascunho.
2. **Checagem:** ligada por padrão, no início e a cada ~6 h com jitter, com download em segundo
   plano. Cada checagem é um GET ao github.com, que vê o IP e sabe que o Fala está rodando. As
   configurações dizem isso e oferecem o toggle para desligar. Nenhum dado do usuário vai no
   request, e não há telemetria.
3. **Instalação:** por clique em "Reiniciar para atualizar" (bandeja e seção Sobre) ou ao sair do
   app. Uma única função no Rust decide se pode instalar e recusa enquanto houver ditado,
   gravação de reunião ou pós-processamento por LLM em andamento. O frontend não chama `install`
   por outro caminho. "Depois" dispensa aquela versão. Sem downgrade: uma versão ruim se corrige
   com uma maior.
4. **Novidades:** mostradas uma vez depois do update. O texto é o mesmo que o `git-cliff` gera
   para o `CHANGELOG.md`, para o corpo do release e para o `notes` do `latest.json`.
5. **Chave:** o par minisign é gerado uma vez pelo mantenedor. A chave privada, com senha, fica no
   gerenciador de senhas mais uma cópia offline guardada em outro lugar. Ela nunca vai para o
   repositório, para workflow, para secret do Actions nem para log, e entra no build só como
   variável de ambiente da sessão. Desde o primeiro release com updater, o app aceita uma segunda
   chave pública, para que a chave possa ser trocada sem perder as instalações.
6. **Build:** local, na máquina Windows do mantenedor, seguindo `docs/RELEASE.md`. Mover o build
   para o GitHub Actions exige outra ADR, porque a chave viraria secret.

### Consequências
* Bom: todo colega recebe as versões seguintes sem reinstalar. O código cliente já existe, então
  não entra nenhuma dependência nova. Sem backend e sem custo.
* Ruim: cada instalação faz um GET ao GitHub a cada ~6 h. A primeira versão com updater ainda é
  instalada à mão. Um release ruim publicado chega a quem atualizar antes de ser retirado, e não
  há rollback. O SmartScreen continua avisando, porque não há assinatura de código.
* Obrigatório:
  * `docs/RELEASE.md` passa a cobrir as variáveis de assinatura no build, o `.sig`, o
    `latest.json` como asset e o teste "atualizar a partir da versão anterior" na validação.
  * `scripts/ci/test_release_config.py` passa a afirmar o updater ligado com o endpoint exato e
    continua barrando assinatura de código.
  * O status da ADR-0008 vira `superseded by ADR-0018` quando esta for aceita, e a 0008 continua
    valendo nos pontos que esta não toca.

## Confirmação
* `scripts/ci/test_release_config.py` checa `pubkey` não vazia, `endpoints` igual à URL acima e
  `createUpdaterArtifacts` ligado, e continua recusando `certificateThumbprint`, `signCommand` e
  `tsp`.
* Um teste do Rust prova que a função de instalação recusa durante ditado, gravação e
  pós-processamento.
* `scripts/check-secrets.sh` e o job `secrets` do CI cobrem a chave privada como qualquer outro
  segredo.
