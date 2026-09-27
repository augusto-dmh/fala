# Fala

Ditado por voz e notas de reunião em português, rodando no seu computador.

> Aperto uma tecla, falo em português, solto, e o texto pronto aparece onde está o cursor, em qualquer app, sem que o áudio da minha voz saia do computador. Nas reuniões e nos vídeos que assisto, o mesmo app grava o que se ouve, transcreve e devolve notas prontas ao final. Roda no meu Windows e no meu Linux, é meu, e um colega pode instalar se quiser.

Fala é um app desktop (Tauri 2 + Rust) e um fork não oficial do [Handy](https://github.com/cjpais/Handy). O reconhecimento de voz do ditado roda localmente (Parakeet via ONNX). A formatação por IA é opcional e recebe só texto. Cada pessoa usa as próprias chaves de API, guardadas no keyring do sistema. Sem chave, o app funciona só local.

## Status

**Pré-alpha.** O repositório acabou de ser criado a partir do Handy e ainda não há instalador do Fala. O que está descrito abaixo em Instalação e Uso é o comportamento planejado para a fase 1.

| Fase | O quê | Estado |
|---|---|---|
| 0 | Spikes e benchmark de ASR em pt-BR | a começar |
| 1 | Ditado no Windows (substitui o Wispr Flow) | a começar |
| 2 | Reunião e vídeos no Windows (substitui o Granola) | depois |
| 3 | Linux GNOME Wayland | depois |

Apetite e critério de fechamento de cada fase: [`ROADMAP.md`](ROADMAP.md).

## Instalação (Windows 11)

1. Baixe o instalador `Fala_<versão>_x64-setup.exe`.
2. Execute. **O Windows vai mostrar o aviso do SmartScreen** ("O Windows protegeu o computador"), porque o instalador ainda não tem assinatura de código ([ADR-0008](docs/decisions/0008-distribuicao-byok-sem-backend-assinatura-e-updater-adiados.md)). Clique em **Mais informações → Executar assim mesmo**.
3. Na primeira execução, escolha o microfone e deixe o app baixar o modelo de voz (algumas centenas de MB, uma vez só).
4. Aperte `Ctrl+Shift+Space`, fale e solte.

Não há atualização automática por enquanto. Para atualizar, instale a versão nova por cima.

## Uso

| Gesto | Efeito |
|---|---|
| Segurar `Ctrl+Shift+Space` | grava enquanto a tecla está pressionada (push-to-talk) |
| Toque duplo | modo mãos-livres, até tocar de novo |
| `Esc` | cancela o ditado atual |

As chaves de API (Gemini para formatação; ElevenLabs e Anthropic para reuniões) são configuradas no app e guardadas no Gerenciador de Credenciais do Windows, nunca em arquivo.

## Privacidade

- O áudio do ditado **nunca** sai da máquina.
- Com a formatação por IA ligada, só vão ao provedor o texto ditado, o nome do app ativo e o seu dicionário. Dá para desligar por app.
- Reuniões só são gravadas quando você clica em "Gravar". O áudio fica no seu disco. A transcrição na nuvem é escolha sua, reunião a reunião.

## Solução de problemas

- **O atalho não responde.** Veja se outro app (Wispr Flow, PowerToys) usa o mesmo atalho e troque um dos dois.
- **O texto não aparece no campo.** Ele fica na área de transferência; cole com `Ctrl+V`. Em terminais, use `Ctrl+Shift+V`.
- **O modelo não baixa.** Confira a conexão e o espaço livre em disco.
- **Logs:** Configurações → Sobre → pasta de logs.

## Desenvolvimento

```bash
bun install
bun run tauri dev                 # app com hot reload
cargo check --workspace           # todo o Rust, incluindo apps/desktop
cargo run -p fala-cli -- --help   # pipeline sem UI
```

- Pré-requisitos e build no Windows: [`docs/dev/build-windows.md`](docs/dev/build-windows.md)
- Mapa do código e invariantes: [`ARCHITECTURE.md`](ARCHITECTURE.md)
- Decisões de arquitetura: [`docs/decisions/`](docs/decisions/)
- Design doc da v1: [`docs/design/2026-10-fala-v1.md`](docs/design/2026-10-fala-v1.md)
- Fluxo, commits e pull requests: [`CONTRIBUTING.md`](CONTRIBUTING.md)
- Regras para agentes de código: [`AGENTS.md`](AGENTS.md)

## Licença e agradecimentos

MIT (ver [`LICENSE`](LICENSE)). Fala começou como fork do [Handy](https://github.com/cjpais/Handy), de CJ Pais, que resolveu o difícil no Windows: hook de teclado, overlay, bandeja e o pipeline de VAD e Parakeet. Fala não é endossado pelo Handy nem afiliado a ele, e não usa o nome nem a marca do Handy. Detalhes em [`NOTICE.md`](NOTICE.md).
