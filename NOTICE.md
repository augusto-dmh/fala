# Avisos

Fala é um fork não oficial de Handy (https://github.com/cjpais/Handy),
Copyright (c) 2025 CJ Pais, distribuído sob a licença MIT (ver `LICENSE`).
O nome, o logo, o ícone e os demais ativos de marca do Handy pertencem ao autor original
e não são usados neste projeto. Fala não é endossado pelo Handy nem afiliado a ele.

## Ponto de partida

| Campo | Valor |
|---|---|
| Upstream | https://github.com/cjpais/Handy |
| Commit | `8f9cf53cd1410cda26beea39ff802ac306e39585` (2026-09-19) |
| `git describe` | `v0.9.7-6-g8f9cf53` |
| Histórico | preservado; o primeiro commit do Fala vem logo depois desse |

## Referências ao Handy que permanecem de propósito

São nomes de terceiros ou proveniência, não marca do Fala. `scripts/check-brand.sh` aceita
só estes padrões:

- o crate `handy-keys` (crates.io), seus caminhos `handy_keys::` e o método `to_handy_string`;
- o CDN de modelos `blob.handy.computer` e a organização `handy-computer` (Hugging Face, GitHub).
  TODO: espelhar os modelos que o Fala usa antes de distribuir para colegas, para não depender
  de infraestrutura alheia sem combinar;
- o link de solução de problemas `handy.computer/docs` (só macOS, que não é alvo);
- citações de issues do upstream em comentários (`cjpais/Handy#NNNN`) e os forks `cjpais/*`
  usados como dependência git (`tao`, `vad-rs`, `rodio`, `hf-hub`).

## Dependências de terceiros

As licenças das dependências Rust são verificadas por `cargo deny check licenses` (`deny.toml`).
TODO: gerar a lista completa com `cargo deny list` e anexá-la aqui antes do primeiro instalador
distribuído (`docs/RELEASE.md`).
