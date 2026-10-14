//! As pessoas das reuniões: um diretório derivado dos nomes de "Quem estava"
//! (`meeting_participants`) e dos nomes aplicados a "Pessoa N" (`meeting_speakers`), sem tabela
//! própria. Os dois já vão para o `.md`, então o diretório atravessa o `reindex` (ADR-0006).
//!
//! A chave de uma pessoa é o nome sem caixa, sem acento, com os espaços colapsados e sem a
//! pontuação das pontas; um e-mail é a sua própria chave, em minúsculas. A mesma pessoa conta
//! uma vez por reunião. Reuniões na lixeira e rascunhos sem nada além do título ficam fora.
//! Juntar duas grafias reescreve os nomes nas reuniões; nenhum apelido fica guardado.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use chrono::{DateTime, FixedOffset};
use fala_meeting::SessionId;
use rusqlite::params;

use crate::meetings::bad;
use crate::{StorageError, Store};

/// Cada nome de uma reunião contada, da reunião mais recente para a mais antiga; dentro de uma
/// reunião, "Quem estava" na ordem digitada e depois os falantes por número.
const APPEARANCES: &str = "
WITH counted AS (
    SELECT m.id, coalesce(m.started_ms, m.created_ms) AS at_ms,
           coalesce(m.started_at, m.created_at) AS at
    FROM meetings m
    WHERE NOT EXISTS (SELECT 1 FROM meeting_trash t WHERE t.meeting_id = m.id)
      AND NOT (m.started_at IS NULL AND m.notes_md IS NULL
               AND trim(m.annotations, ' ' || char(9, 10, 13)) = '')
)
SELECT c.id, c.at, p.name, c.at_ms, 0 AS source, p.position AS n
FROM counted c JOIN meeting_participants p ON p.meeting_id = c.id
UNION ALL
SELECT c.id, c.at, s.name, c.at_ms, 1, s.person
FROM counted c JOIN meeting_speakers s ON s.meeting_id = c.id
WHERE s.name IS NOT NULL
ORDER BY 4 DESC, 1 DESC, 5, 6";

/// Uma pessoa do diretório.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryPerson {
    /// [`person_key`] do nome: o que junta as grafias.
    pub key: String,
    /// A grafia da reunião mais recente.
    pub name: String,
    /// Quando a reunião mais recente com ela começou (ou foi criada, num rascunho).
    pub last_meeting_at: DateTime<FixedOffset>,
    /// Em quantas reuniões ela está.
    pub meetings: u32,
}

/// A letra sem acento, para as letras do português e do espanhol; o resto fica como está. Um
/// acento solto (texto colado em NFD) sai em [`fold`].
fn unaccent(c: char) -> char {
    match c {
        'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'ç' => 'c',
        'ñ' => 'n',
        _ => c,
    }
}

/// O texto em minúsculas, sem acento, com os espaços colapsados e sem a pontuação das pontas.
pub(crate) fn fold(text: &str) -> String {
    let lower: String = text
        .chars()
        .filter(|c| !('\u{300}'..='\u{36f}').contains(c))
        .flat_map(char::to_lowercase)
        .map(unaccent)
        .collect();
    let words = lower.split_whitespace().collect::<Vec<_>>().join(" ");
    words
        .trim_matches(|c: char| !c.is_alphanumeric())
        .to_string()
}

/// A chave que junta as grafias de uma pessoa: "Ana", " ana " e "Aná" dão `ana`, "Ana Souza"
/// não. Um e-mail é a sua própria chave, em minúsculas. `None` quando não sobra nada.
pub fn person_key(name: &str) -> Option<String> {
    let trimmed = name.trim();
    let email = trimmed
        .split_once('@')
        .is_some_and(|(user, domain)| !user.is_empty() && domain.contains('.'));
    let key = if email && !trimmed.contains(char::is_whitespace) {
        trimmed
            .trim_matches(|c: char| !c.is_alphanumeric())
            .to_lowercase()
    } else {
        fold(trimmed)
    };
    (!key.is_empty()).then_some(key)
}

/// Um nome numa reunião contada.
struct Appearance {
    meeting: SessionId,
    at: String,
    name: String,
    key: String,
}

impl Store {
    fn appearances(&self) -> Result<Vec<Appearance>, StorageError> {
        let mut stmt = self.conn.prepare(APPEARANCES)?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (id, at, name) = row?;
            let Some(key) = person_key(&name) else {
                continue;
            };
            let meeting = id
                .parse()
                .map_err(|_| StorageError::Db(bad(0, format!("id de sessão inválido {id:?}"))))?;
            out.push(Appearance {
                meeting,
                at,
                name,
                key,
            });
        }
        Ok(out)
    }

    /// As pessoas das reuniões, da que esteve numa reunião mais recentemente para a que esteve
    /// há mais tempo.
    pub fn meeting_directory(&self) -> Result<Vec<DirectoryPerson>, StorageError> {
        let mut people: Vec<DirectoryPerson> = Vec::new();
        let mut index: HashMap<String, usize> = HashMap::new();
        let mut seen: HashSet<(String, SessionId)> = HashSet::new();
        for appearance in self.appearances()? {
            if !seen.insert((appearance.key.clone(), appearance.meeting)) {
                continue;
            }
            if let Some(&i) = index.get(&appearance.key) {
                people[i].meetings += 1;
                continue;
            }
            let last_meeting_at = DateTime::parse_from_rfc3339(&appearance.at).map_err(|e| {
                StorageError::Db(bad(1, format!("data inválida {:?}: {e}", appearance.at)))
            })?;
            index.insert(appearance.key.clone(), people.len());
            people.push(DirectoryPerson {
                key: appearance.key,
                name: appearance.name.trim().to_string(),
                last_meeting_at,
                meetings: 1,
            });
        }
        Ok(people)
    }

    /// As reuniões contadas em que está a pessoa de chave `key`, da mais recente para a mais
    /// antiga.
    pub fn person_meeting_ids(&self, key: &str) -> Result<Vec<SessionId>, StorageError> {
        let mut ids: Vec<SessionId> = Vec::new();
        let mut seen: HashSet<SessionId> = HashSet::new();
        for appearance in self.appearances()? {
            if appearance.key == key && seen.insert(appearance.meeting) {
                ids.push(appearance.meeting);
            }
        }
        Ok(ids)
    }

    /// Troca por `name`, em toda reunião (lixeira inclusive), cada nome de chave `key` em "Quem
    /// estava" e em "Pessoa N", numa transação; o `.md` de cada uma acompanha. Juntar "Ana" com
    /// "Ana Souza" é trocar a chave `ana` pelo nome "Ana Souza": numa reunião com os dois, "Quem
    /// estava" fica com um só. Devolve as sessões que mudaram.
    pub fn rename_person(&mut self, key: &str, name: &str) -> Result<Vec<SessionId>, StorageError> {
        let tx = self.conn.transaction()?;
        let mut lists: BTreeMap<String, Vec<String>> = BTreeMap::new();
        {
            let mut stmt = tx.prepare(
                "SELECT meeting_id, name FROM meeting_participants ORDER BY meeting_id, position",
            )?;
            let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
            for row in rows {
                let (id, typed): (String, String) = row?;
                lists.entry(id).or_default().push(typed);
            }
        }
        let target = person_key(name);
        let mut changed: BTreeSet<String> = BTreeSet::new();
        for (id, names) in lists {
            if !names.iter().any(|n| person_key(n).as_deref() == Some(key)) {
                continue;
            }
            // Only the renamed person loses a repeat; anyone else stays as typed.
            let mut kept: Vec<String> = Vec::new();
            let mut has_target = false;
            for typed in &names {
                let typed = if person_key(typed).as_deref() == Some(key) {
                    name
                } else {
                    typed
                };
                if target.is_some() && person_key(typed) == target {
                    if has_target {
                        continue;
                    }
                    has_target = true;
                }
                kept.push(typed.to_string());
            }
            if kept == names {
                continue;
            }
            tx.execute(
                "DELETE FROM meeting_participants WHERE meeting_id = ?1",
                [&id],
            )?;
            for (index, typed) in kept.iter().enumerate() {
                tx.execute(
                    "INSERT INTO meeting_participants (meeting_id, position, name) VALUES (?1, ?2, ?3)",
                    params![id, index as i64 + 1, typed],
                )?;
            }
            changed.insert(id);
        }
        type Speaker = (String, u32, Option<String>, Option<String>);
        let speakers: Vec<Speaker> = {
            let mut stmt = tx
                .prepare("SELECT meeting_id, person, name, suggested_name FROM meeting_speakers")?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })?;
            rows.collect::<Result<_, _>>()?
        };
        let renamed = |typed: &Option<String>| {
            typed
                .as_deref()
                .is_some_and(|t| t != name && person_key(t).as_deref() == Some(key))
        };
        for (id, person, typed, suggested) in speakers {
            if renamed(&typed) {
                tx.execute(
                    "UPDATE meeting_speakers SET name = ?3 WHERE meeting_id = ?1 AND person = ?2",
                    params![id, person, name],
                )?;
                changed.insert(id.clone());
            }
            // A suggestion applied later must not bring the old spelling back.
            if renamed(&suggested) {
                tx.execute(
                    "UPDATE meeting_speakers SET suggested_name = ?3
                     WHERE meeting_id = ?1 AND person = ?2",
                    params![id, person, name],
                )?;
            }
        }
        tx.commit()?;
        let mut ids = Vec::with_capacity(changed.len());
        for id in changed {
            let id: SessionId = id
                .parse()
                .map_err(|_| StorageError::Db(bad(0, format!("id de sessão inválido {id:?}"))))?;
            self.after_meeting_change(id);
            ids.push(id);
        }
        log::debug!("pessoa renomeada em {} reuniões", ids.len());
        Ok(ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spellings_of_a_name_share_a_key() {
        for name in ["Ana", "ana", "Aná", "  ANA  ", "Ana.", "(Ana)"] {
            assert_eq!(person_key(name).as_deref(), Some("ana"), "{name:?}");
        }
        assert_eq!(person_key("Ana   Souza").as_deref(), Some("ana souza"));
        assert_eq!(
            person_key("João Gonçalves").as_deref(),
            Some("joao goncalves")
        );
        assert_eq!(person_key("Iñaki Muñoz").as_deref(), Some("inaki munoz"));
        assert_eq!(person_key("Ana\u{301}").as_deref(), Some("ana"));
        assert_eq!(person_key("@Ana").as_deref(), Some("ana"));
        assert_ne!(person_key("Ana"), person_key("Ana Souza"));
    }

    #[test]
    fn an_email_is_its_own_key() {
        assert_eq!(
            person_key(" Ana.Souza@Example.com ").as_deref(),
            Some("ana.souza@example.com")
        );
        assert_eq!(person_key("<ana@x.com>").as_deref(), Some("ana@x.com"));
        assert_ne!(person_key("ana@x.com"), person_key("Ana"));
    }

    #[test]
    fn blank_or_punctuation_has_no_key() {
        assert_eq!(person_key(""), None);
        assert_eq!(person_key("  "), None);
        assert_eq!(person_key(" - . "), None);
    }
}
