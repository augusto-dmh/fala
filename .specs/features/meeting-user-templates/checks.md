# meeting-user-templates checks

Profile: light
Plan: `.specs/features/meeting-user-templates/plan.md`

Todas as provas Rust rodam com `CARGO_TARGET_DIR=C:\f\meeting-user-templates`; nada de rede,
chave ou microfone.

**C1** - Pasta ausente ou vazia: nenhum template do usuário (AC 1)
Proof: `cargo test -p fala-notes template::tests::missing_or_empty_folder_has_no_user_templates -- --exact`

**C2** - Um válido (com BOM) e um inválido: um template e um erro com o nome do arquivo; o
`LEIAME.txt` é ignorado (AC 2)
Proof: `cargo test -p fala-notes template::tests::invalid_file_becomes_an_error_and_the_valid_one_loads -- --exact`

**C3** - `id` de embutido ou repetido perde, com erro (AC 3)
Proof: `cargo test -p fala-notes template::tests::colliding_id_loses_to_the_builtin_and_to_the_earlier_file -- --exact`

**C4** - Duplicar: `geral-copia`, "Reunião geral (cópia)", depois `geral-copia-2`; o arquivo passa
em `from_json`; `LEIAME.txt` criado e nunca reescrito (AC 4)
Proof: `cargo test -p fala-notes template::tests::duplicate_gets_a_new_id_and_name_and_loads_back -- --exact`

**C5** - `meeting_templates` devolve embutidos e do usuário com a origem e os erros;
`find_template` acha o do usuário e mantém o embutido numa colisão (AC 5)
Proof: `cargo test -p fala --lib meeting::pipeline::tests::meeting_templates_lists_builtin_and_user_templates -- --exact` · `grep -n "duplicateMeetingTemplate\|openMeetingTemplatesFolder\|source: TemplateSource" src/bindings.ts`

**C6** - Dois grupos com template do usuário, busca acima de 5, lista em Reuniões com os botões e
os erros, releitura no foco, strings em pt e en (AC 6)
Proof: `bun src/components/meeting/userTemplates.test.tsx` (UT-groups, UT-search, UT-settings, UT-i18n)
