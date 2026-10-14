//! As séries de reuniões: "Weekly produto 13/10" e "Weekly produto 20/10" são a mesma, porque o
//! título sem as datas e a numeração final dá a mesma chave. Nada é guardado: renomear a reunião
//! muda a série na hora, e o `reindex` não tem o que reconstruir (ADR-0006).

use fala_meeting::SessionId;

use crate::meeting_directory::fold;
use crate::meeting_trash::parse_id;
use crate::{StorageError, Store};

/// Os meses como `fold` os deixa, para "13 de outubro".
const MONTHS: &str = "janeiro fevereiro marco abril maio junho julho agosto setembro outubro \
    novembro dezembro jan fev mar abr mai jun jul ago set out nov dez";

/// O título padrão de uma reunião sem nome não faz série.
const DEFAULT_TITLES: [&str; 2] = ["reuniao", "meeting"];

/// A série de um título.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Series {
    /// O título sem datas nem numeração, sem caixa nem acento: o que junta as reuniões.
    pub key: String,
    /// O mesmo, na grafia de quem digitou.
    pub name: String,
}

fn number(s: &str, max: u32) -> bool {
    !s.is_empty() && s.len() <= 4 && s.parse::<u32>().is_ok_and(|n| (1..=max).contains(&n))
}

/// `13/10`, `13/10/2026`, `13-10-2026`, `13.10.26` ou `2026-10-13`. Dois números só com `/`:
/// "Release 1.2" e "Sprint 3-4" são versões e intervalos, não datas.
fn is_date(word: &str) -> bool {
    ['/', '-', '.'].iter().any(|&sep| {
        let parts: Vec<&str> = word.split(sep).collect();
        match parts.as_slice() {
            [y, m, d] if y.len() == 4 => number(y, 9999) && number(m, 12) && number(d, 31),
            [d, m] => sep == '/' && number(d, 31) && number(m, 12),
            [d, m, y] => number(d, 31) && number(m, 12) && number(y, 9999),
            _ => false,
        }
    })
}

/// O que vem antes da numeração final, já sem pontas: "nº", "n°" e "n." (`n`), "no", e "-" ou
/// "#" soltos (vazios depois de `fold`).
fn is_marker(folded: &str) -> bool {
    matches!(folded, "" | "n" | "no" | "nº")
}

/// As palavras do título que ficam na série, na grafia original.
fn series_words(title: &str) -> Vec<&str> {
    let words: Vec<&str> = title.split_whitespace().collect();
    let folded: Vec<String> = words.iter().map(|w| fold(w)).collect();
    let mut keep = vec![true; words.len()];
    let mut i = 0;
    while i < words.len() {
        if is_date(&folded[i]) {
            keep[i] = false;
        } else if number(&folded[i], 31)
            && folded.get(i + 1).is_some_and(|w| w == "de")
            && folded
                .get(i + 2)
                .is_some_and(|w| MONTHS.split_whitespace().any(|m| m == w))
        {
            // "13 de outubro" e, se vier, "de 2026".
            let mut end = i + 3;
            if folded.get(end).is_some_and(|w| w == "de")
                && folded
                    .get(end + 1)
                    .is_some_and(|w| w.len() == 4 && number(w, 9999))
            {
                end += 2;
            }
            keep[i..end].iter_mut().for_each(|k| *k = false);
            i = end;
            continue;
        }
        i += 1;
    }
    let mut kept: Vec<usize> = (0..words.len()).filter(|&i| keep[i]).collect();
    // A numeração final ("#12", "nº 12", "- 12") e a pontuação que sobrar no fim.
    while let Some(&last) = kept.last() {
        let hash = words[last]
            .strip_prefix('#')
            .is_some_and(|n| number(n, u32::MAX));
        let digits = number(&folded[last], u32::MAX);
        let before = kept.len().checked_sub(2).map(|i| folded[kept[i]].as_str());
        if hash || folded[last].is_empty() {
            kept.pop();
        } else if digits && before.is_some_and(is_marker) {
            kept.truncate(kept.len() - 2);
        } else {
            break;
        }
    }
    kept.into_iter().map(|i| words[i]).collect()
}

/// A série de um título: `None` para o vazio, o padrão ou uma chave com menos de 3 letras.
pub fn series_of(title: &str) -> Option<Series> {
    let name = series_words(title).join(" ");
    let name = name
        .trim_matches(|c: char| !c.is_alphanumeric())
        .to_string();
    let key = fold(&name);
    (key.chars().count() >= 3 && !DEFAULT_TITLES.contains(&key.as_str()))
        .then_some(Series { key, name })
}

impl Store {
    /// As sessões fora da lixeira com a série `key`, da que começou por último para a primeira.
    pub fn series_meetings(&self, key: &str) -> Result<Vec<SessionId>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT m.id, m.title FROM meetings m
             WHERE NOT EXISTS (SELECT 1 FROM meeting_trash t WHERE t.meeting_id = m.id)
             ORDER BY coalesce(m.started_ms, m.created_ms) DESC, m.id DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut ids = Vec::new();
        for row in rows {
            let (id, title) = row?;
            if series_of(&title).is_some_and(|s| s.key == key) {
                ids.push(parse_id(&id)?);
            }
        }
        Ok(ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(title: &str) -> Option<String> {
        series_of(title).map(|s| s.key)
    }

    #[test]
    fn dates_and_final_numbers_leave_the_key() {
        for (title, want) in [
            ("Weekly produto 13/10", "weekly produto"),
            ("Weekly produto 20/10/2026", "weekly produto"),
            ("weekly  Produto - 2026-10-13", "weekly produto"),
            ("Weekly produto 13 de outubro", "weekly produto"),
            ("Weekly produto, 13 de out. de 2026", "weekly produto"),
            ("Retro #12", "retro"),
            ("Retro nº 12", "retro"),
            ("Retro - 12", "retro"),
            ("Retrô #13", "retro"),
            ("1:1 Ana", "1:1 ana"),
            ("Sprint 12", "sprint 12"),
            ("Release 2.0", "release 2.0"),
            ("Release 1.2", "release 1.2"),
            ("Sprint 3-4", "sprint 3-4"),
            ("Daily 13.10.2026", "daily"),
        ] {
            assert_eq!(key(title).as_deref(), Some(want), "{title:?}");
        }
        assert_ne!(key("1:1 Ana"), key("1:1 Bruno"));
    }

    #[test]
    fn no_series_for_blank_default_or_short() {
        for title in [
            "",
            "  ",
            "Reunião",
            "reuniao 13/10",
            "Meeting",
            "AB 13/10",
            "#4",
        ] {
            assert_eq!(key(title), None, "{title:?}");
        }
    }

    #[test]
    fn the_name_keeps_the_typed_spelling() {
        let series = series_of("Weekly Produto — 13/10").unwrap();
        assert_eq!(series.name, "Weekly Produto");
    }
}
