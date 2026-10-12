# app-styles — plano

Profile: light · Fonte: pitch `fase-4-estilo-por-app` (W15/W16 da matriz de 2026-10-11).

## O que já existe

`destination_for` (#93) põe `<destination>` (email/chat/prompt/editor) na mensagem pelo nome do
app; o nível de limpeza (#103) entra depois do prompt fixo; o dicionário fecha o
`systemInstruction`. Não há escolha do usuário por app.

## Mudança

- `fala-postproc`: `FormatContext.style: Option<&str>`; `Postprocessor::with_app_styles(app,
  instrução)`, app comparado sem caixa; `system_text` acrescenta `STYLE_INSTRUCTION` + a
  instrução **no fim**, depois do dicionário, para o prefixo comum seguir cacheado. App sem
  estilo: payload idêntico ao de antes.
- `apps/desktop`: módulo `app_styles` com `WritingStyle {id, name, instruction}` e
  `AppStyle {app, style}`; settings `writing_styles` (quatro prontos: Mensagem, E-mail,
  Documento, Código e terminal, alinhados aos destinos) e `app_styles` (vazio). Comandos
  `update_writing_styles` (remove o mapeamento de estilo apagado) e `update_app_styles` (nome do
  app pelo `app_name_from_exe_path`, um estilo por app, estilo desconhecido cai).
  `formatter_at` passa `instructions(settings)`.
- UI: `AppStyles` na tela de IA, depois de "Apps sem IA": app + estilo + Adicionar, chips
  `app → estilo`, lista de estilos com nome e instrução editáveis (salva no blur), novo e remover.

## Decisões

- App desconhecido e app sem mapeamento: comportamento atual, sem estilo extra (orquestrador).
- Mapeamento só explícito: os apps conhecidos já recebem o `<destination>`, e um estilo pronto
  por padrão só repetiria a linha do destino.
- O estilo vence o destino quando os dois existem ("takes precedence").
- Os nomes e instruções prontos são dados do usuário em pt-BR, não strings de UI.
- Fora: "Polish automático"; override por ditado na bandeja (fatia seguinte).

## Portas de uma via

- Literais guardados: `writing_styles[].{id,name,instruction}`, `app_styles[].{app,style}`, ids
  dos prontos `message|email|document|code`.
