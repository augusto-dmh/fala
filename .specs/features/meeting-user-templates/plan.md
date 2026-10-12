# meeting-user-templates

## Problem

O Fala tem dois templates de notas embutidos (`geral`, `um-a-um`) e o comentário de
`crates/notes/src/template.rs` já diz que templates da empresa chegam como templates do usuário
(D10), mas não existe de onde lê-los. É a fatia 2b do `meeting-notes-context` (G36).

## Flow

1. `<dados>/templates/*.json` (portátil: `Data/templates`), relida a cada uso, sem watcher
2. `load_user_templates` → templates + erros `{arquivo, erro}`; `meeting_templates` devolve os
   dois com a origem; `find_template` procura nos embutidos e depois na pasta
3. Reuniões → "Templates de notas": [Abrir a pasta], [Duplicar para editar] nos embutidos, erros;
   relida no foco. Os dois seletores viram `TemplateSelect` (grupos e busca acima de 5)

## Landing

| One-way door | Literal shape | Alternative rejected |
| --- | --- | --- |
| 1. pasta e formato (ADR-0006) | `<dados>/templates/*.json`, o mesmo JSON dos embutidos (schema 1), `LEIAME.txt` em pt | um template por linha no SQLite: não se edita fora do app |
| 2. comando `meeting_templates` muda de forma | `MeetingTemplates {templates, errors}`, `MeetingTemplate` ganha `purpose` e `source` | um segundo comando só para os erros: duas leituras da pasta por abertura |

- Nada novo no payload (ADR-0016): o template do usuário entra no lugar do embutido.

## Decisions

No corpo do PR.

## Criteria

1. WHEN a pasta não existe ou está vazia THEN SHALL haver só os embutidos
2. WHEN a pasta tem um JSON válido e um inválido THEN SHALL haver um template e um erro com o nome do arquivo
3. WHEN um arquivo usa o `id` de um embutido ou de um arquivo anterior THEN o primeiro SHALL ficar e o arquivo SHALL virar erro
4. WHEN "Duplicar para editar" THEN SHALL nascer um JSON com `id` e nome novos que `from_json` aceita, mais o `LEIAME.txt`
5. `meeting_templates` SHALL devolver as duas origens e os erros; `find_template` SHALL achar o template do usuário
6. Os seletores SHALL mostrar dois grupos com template do usuário e busca acima de 5; Reuniões SHALL listar os templates com [Abrir a pasta], [Duplicar para editar] e os erros; toda string nova em pt e en
