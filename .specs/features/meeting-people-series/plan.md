# meeting-people-series — pessoas e séries das reuniões (PRs 1-4)

Profile: light · fase 4 (semana 1, sem LLM) · ADR-0006, 0016, 0017 · nenhuma contrariada

- **PR 1**: `meeting_directory` (consulta sobre `meeting_participants` e nomes de
  `meeting_speakers`, sem tabela); chave sem caixa/acento/espaços/pontuação das pontas, e-mail
  como chave; uma vez por reunião; fora lixeira e rascunho vazio; "Reuniões | Pessoas".
- **PR 2**: juntar e renomear; sugestão ao digitar. **PR 3**: série pelo título, "Mesma série".
  **PR 4**: template padrão por série.

Decisions (PR 1): rascunho vazio = nunca gravou, sem pauta e sem notas; grafia = a da reunião
mais recente ("Quem estava" antes do falante); a busca filtra nomes no front, sem acento.

Decisions (PR 2): juntar = renomear a chave da origem para a grafia do destino; reescreve também
lixeira e rascunhos (restaurar não traz a grafia velha) e a sugestão pendente; a contagem dita é
a de Pessoas; só a pessoa renomeada perde repetidos; renomear para o nome de outra pessoa pede a
confirmação de juntar.

Decisions (PR 3): a série vem do título em toda abertura da sessão (nada guardado); "anteriores"
= começaram antes desta; "ver todas" filtra a lista da página com "Série “…”" [Limpar]; datas
inválidas (mês 0, dia 32) não saem do título ("Release 2.0" fica); "1:1" sozinho é série.
