# meeting-notes-polish

## Problem

A nota de reunião já é um documento (IA em cinza, lupa, tocar trecho) e já aceita o nome sugerido
pelo LLM (`meeting-notes-context`, #112/#114). Falta a parte 2 do painel: a nota não é editável e
"Gerar notas" sobrescreve sem perguntar; "Pessoa N" só ganha nome por sugestão, sem renomear à mão
nem desfazer; o template só aparece na hora de gerar; não há aviso para colar na call, nem lembrete
de consentimento a cada início, nem busca na transcrição; e parar → transcrever → gerar ainda pede
o clique de gerar, sem estados nem erro que diga que nada se perdeu.

## Já existe (cortado da definição)

- Nome aplicado na transcrição, no payload (`Speaker::Named`) e no "Copiar Markdown"; a tabela
  `meeting_speakers` com `name`; transcrever de novo apaga os nomes.
- Transcrever sozinho ao parar quando há a chave da ElevenLabs (`MeetingManager::finish`).

## Fatias (três PRs empilhados; o CI barra acima de 1000 linhas)

- **PR 1** `feat/meeting-notes-polish`: renomear "Pessoa N" (e desfazer) e, numa sessão com
  notas, oferecer "Gerar de novo" depois do nome trocado.
- **PR 2** `feat/meeting-notes-polish-2` (sobre 1): nota editável e confirmação antes de regenerar
  uma nota editada com o mesmo template.
- **PR 3** `feat/meeting-notes-polish-3` (sobre 2): template no rascunho e na gravação, "Copiar
  aviso", chip de consentimento de 5 s, busca sem acento, "Copiar transcrição", parar →
  transcrever → gerar encadeados (acima de um mínimo) com estados e erro.

## Flow (PRs 1 e 2)

1. Clique no rótulo "Pessoa N" da transcrição → campo → Enter/blur → `set_meeting_speaker_name`
   → `meeting_speakers.name` (vazio apaga o nome; a sugestão pendente daquela pessoa some)
2. "Editar" na nota → textarea com o texto visível (sem marcador, ponteiros nem âncora `^aN`) →
   cada mudança, no máximo 2 s depois (`AnnotationSaver`), `mergeEdit(base, texto)` →
   `save_meeting_notes` → `notes_md` e `meeting_note_edits`
3. `mergeEdit`: linha igual a uma linha da nota de antes volta com o Markdown original (marcador
   e ponteiros); linha mudada ou nova vira texto do usuário (sem marcador, sem ponteiro)
4. "Gerar notas" numa nota editada, com o mesmo template → confirmação na página →
   `generate_meeting_notes(id, template, replace_edited: true)`; sem a confirmação o backend
   devolve `notes_edited`. Outro template não pergunta (é uma versão nova, como no Granola)
5. Uma geração grava a nota e apaga a marca de editada
6. Nome trocado numa sessão que já tem notas → "Nome trocado. Gerar as notas de novo?" [Gerar de
   novo] [Agora não]; gerar passa pela regra do passo 4

## Landing

| One-way door | Literal shape | Alternative rejected |
| --- | --- | --- |
| 1. tabela nova (ADR-0006, aditiva) | `meeting_note_edits(meeting_id TEXT PRIMARY KEY REFERENCES meetings(id) ON DELETE CASCADE, edited_at TEXT NOT NULL)`, `CREATE TABLE IF NOT EXISTS` a cada abertura, sem subir `user_version` (como `meeting_speakers`) | coluna `notes_edited_at` em `meetings`: ALTER + `user_version` 4 num arquivo que outro painel também mexe; hash da nota gerada: não diz quando foi editada |
| 2. comando `generate_meeting_notes(id, template_id, replace_edited: bool)` e erro `{"kind":"notes_edited"}` | a UI só manda `true` depois do clique em "Gerar de novo" | confirmar só na UI: um segundo caminho (CLI, MCP) sobrescreveria sem perguntar |
| 3. nome digitado no payload (ADR-0016) | o nome aplicado sai como rótulo do falante, como o sugerido; um nome com `@` sai como "Pessoa N" | recusar o nome com `@` na UI: a pessoa veria um erro por algo que só importa no envio |

- Nothing else in this change is hard to reverse

## Decisions (registradas no PR)

- Editar = textarea do texto visível da nota, não editor rico: a nota volta ao documento cinza/preto
  ao "Concluir". A linha gerada que a pessoa muda perde o marcador e os ponteiros (regra do Granola).
- Renomear vale para a sessão (transcrição, próximas notas, Markdown); não reescreve a nota já gerada.
- Nome com `@` fica na transcrição como digitado, mas nunca vai ao LLM (ADR-0016).
- Pesquisa REA (`research/23` § 1): confirmar só com o mesmo template, no texto do Granola
  ("Você editou estas notas. Gerar de novo substitui as suas edições, e isso não pode ser
  desfeito."); oferecer gerar de novo depois de um nome trocado; encadear só acima de 2.500
  caracteres e 3 min de transcrição; busca sem acento; "Copiar transcrição" como "Nome: texto"
  por turno. O "/" que aplica o template como esqueleto fica de fora.

## Criteria

1. WHEN a sessão é aberta THEN `meeting_note_edits` SHALL existir, e apagar a reunião apaga a linha
2. WHEN a nota é salva pela edição THEN `notes_md` SHALL mudar e `meeting_notes_edited` ser `true`; uma geração SHALL zerar a marca
3. WHEN a nota editada é regenerada com o mesmo template sem `replace_edited` THEN SHALL voltar `notes_edited` sem chamar o LLM; com outro template, SHALL seguir
4. WHEN "Pessoa 2" recebe um nome à mão THEN a transcrição, a próxima `notes_input` e o Markdown SHALL usar o nome, e a sugestão pendente dela some; nome vazio SHALL voltar a "Pessoa 2"
5. WHEN o nome aplicado contém `@` THEN o payload SHALL rotular "Pessoa N"
6. WHEN uma linha gerada é editada THEN ela SHALL perder o marcador e os ponteiros; as linhas iguais SHALL manter os dois
7. The UI SHALL pedir confirmação antes de regenerar uma nota editada com o mesmo template, oferecer gerar de novo depois de um nome trocado numa sessão com notas, e toda string nova SHALL estar em pt e en
