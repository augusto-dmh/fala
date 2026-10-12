# meeting-user-templates

## Problem

O Fala tem dois templates de notas embutidos (`geral`, `um-a-um`) e o comentário de
`crates/notes/src/template.rs` já diz que templates da empresa chegam como templates do usuário
(D10), mas não existe de onde lê-los. É a fatia 2b do `meeting-notes-context` (G36).

## Flow

1. `<dados do Fala>/templates/*.json` (portátil: `Data/templates`), lida a cada uso, sem watcher
2. `load_user_templates(dir)` → templates em ordem de nome do arquivo + erros `{arquivo, erro}`;
   `id` vazio, igual a um embutido ou repetido perde, com erro
3. `meeting_templates` → `{templates: [{id, name, purpose, source}], errors}`; `find_template`
   procura nos embutidos e depois na pasta, então "Gerar notas", o template da sessão, as notas
   depois do parar e o "Copiar Markdown" aceitam um template do usuário
4. Reuniões → "Templates de notas": [Abrir a pasta] (cria a pasta e o `LEIAME.txt`), cada
   template com a origem, [Duplicar para editar] nos embutidos (copia para `<id>-copia.json` com
   nome "<nome> (cópia)", abre a pasta) e os arquivos quebrados como "arquivo.json: erro"
5. A página relê a lista ao montar e quando a janela ganha o foco
6. Os dois seletores viram `TemplateSelect`: grupos "Embutidos" / "Seus templates" quando há
   template do usuário, e busca por nome e propósito (sem acento) com mais de 5

## Landing

| One-way door | Literal shape | Alternative rejected |
| --- | --- | --- |
| 1. pasta e formato (ADR-0006) | `<dados>/templates/*.json`, o mesmo JSON dos embutidos (schema 1), `LEIAME.txt` em pt | um template por linha no SQLite: não se edita fora do app |
| 2. comando `meeting_templates` muda de forma | `MeetingTemplates {templates, errors}`, `MeetingTemplate` ganha `purpose` e `source` | um segundo comando só para os erros: duas leituras da pasta por abertura |

- Nada novo no payload (ADR-0016): o template do usuário entra no lugar do embutido.

## Decisions (registradas no PR)

- Colisão de `id` com um embutido: o embutido fica e o arquivo aparece como erro (definição).
- A cópia se chama "<nome> (cópia)" em pt, como o conteúdo dos embutidos, em qualquer idioma da UI.
- `id` da cópia: `<id>-copia`, depois `-copia-2`…; nunca sobrescreve um arquivo.
- JSON salvo com BOM (Bloco de Notas) é aceito.
- Os grupos só aparecem quando há template do usuário; sem ele o seletor fica como antes.
- Um template do usuário apagado depois de escolhido: as notas depois do parar e o Markdown caem
  no `geral`, como já faziam com um id desconhecido; "Gerar notas" com ele responde "Template
  desconhecido".

## Criteria

1. WHEN a pasta não existe ou está vazia THEN SHALL haver só os embutidos
2. WHEN a pasta tem um JSON válido e um inválido THEN SHALL haver um template e um erro com o nome do arquivo
3. WHEN um arquivo usa o `id` de um embutido ou de um arquivo anterior THEN o primeiro SHALL ficar e o arquivo SHALL virar erro
4. WHEN "Duplicar para editar" THEN SHALL nascer um JSON com `id` e nome novos que `from_json` aceita, mais o `LEIAME.txt`
5. `meeting_templates` SHALL devolver as duas origens e os erros; `find_template` SHALL achar o template do usuário
6. Os seletores SHALL mostrar dois grupos com template do usuário e busca acima de 5; Reuniões SHALL listar os templates com [Abrir a pasta], [Duplicar para editar] e os erros; toda string nova em pt e en
