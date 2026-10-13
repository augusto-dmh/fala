//! Os ponteiros das notas depois de um trecho apagado: a transcrição é renumerada 1..n, e cada
//! `[[#^sN|mm:ss]]` passa a citar o número novo do mesmo trecho; o que citava um trecho que saiu
//! vira [`ERASED_POINTER`]. Nada mais nas notas muda.

use std::collections::BTreeMap;

/// O que fica no lugar de um ponteiro para um trecho apagado.
pub const ERASED_POINTER: &str = "(trecho apagado)";

const OPEN: &str = "[[#^s";

/// Reescreve os ponteiros de `markdown` por `map` (número antigo → novo, `None` se o trecho
/// saiu). Um número fora do mapa fica como está. Devolve o texto e quantas linhas citavam um
/// trecho apagado.
pub fn remap_pointers(markdown: &str, map: &BTreeMap<u32, Option<u32>>) -> (String, usize) {
    let mut lines_hit = 0;
    let out: Vec<String> = markdown
        .split('\n')
        .map(|line| {
            let (line, hit) = remap_line(line, map);
            lines_hit += usize::from(hit);
            line
        })
        .collect();
    (out.join("\n"), lines_hit)
}

fn remap_line(line: &str, map: &BTreeMap<u32, Option<u32>>) -> (String, bool) {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    let mut hit = false;
    while let Some(at) = rest.find(OPEN) {
        out.push_str(&rest[..at]);
        let after = &rest[at + OPEN.len()..];
        let digits = after.len() - after.trim_start_matches(|c: char| c.is_ascii_digit()).len();
        let close = after.find("]]");
        let pointer = after[..digits]
            .parse::<u32>()
            .ok()
            .zip(close)
            .filter(|_| after[digits..].starts_with('|'));
        match pointer.map(|(n, close)| (map.get(&n).copied(), n, close)) {
            Some((Some(None), _, close)) => {
                out.push_str(ERASED_POINTER);
                hit = true;
                rest = &after[close + 2..];
            }
            Some((Some(Some(new)), old, close)) if new != old => {
                out.push_str(&format!("{OPEN}{new}{}]]", &after[digits..close]));
                rest = &after[close + 2..];
            }
            _ => {
                out.push_str(OPEN);
                rest = after;
            }
        }
    }
    out.push_str(rest);
    (out, hit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pointers_follow_their_segment_and_erased_ones_say_so() {
        // s2 e s3 saíram e o marcador "[trecho apagado]" entrou como s2: o 4 virou 3.
        let map = BTreeMap::from([
            (1, Some(1)),
            (2, None),
            (3, None),
            (4, Some(3)),
            (12, Some(10)),
        ]);
        let notes = "## Notas\n\n\
                     - Data: 15/11. [[#^s4|01:05]] [[#^a2|a2]] <!-- fala:ia -->\n\
                     - Preço. [[#^s2|00:40]] [[#^s12|1:02:05]] <!-- fala:ia -->\n\
                     - Só apagado. [[#^s3|00:50]] <!-- fala:ia -->\n\
                     - Minha nota, sem ponteiro [[#^s99|x]] e [[#^sx|y]] [[#^s1|00:00]]";
        let (out, lines) = remap_pointers(notes, &map);
        assert_eq!(
            out,
            "## Notas\n\n\
             - Data: 15/11. [[#^s3|01:05]] [[#^a2|a2]] <!-- fala:ia -->\n\
             - Preço. (trecho apagado) [[#^s10|1:02:05]] <!-- fala:ia -->\n\
             - Só apagado. (trecho apagado) <!-- fala:ia -->\n\
             - Minha nota, sem ponteiro [[#^s99|x]] e [[#^sx|y]] [[#^s1|00:00]]"
        );
        assert_eq!(lines, 2);
        assert_eq!(
            remap_pointers("sem ponteiros", &map),
            ("sem ponteiros".into(), 0)
        );
        // Um ponteiro quebrado no fim não trava a leitura.
        assert_eq!(remap_pointers("[[#^s4|01", &map).0, "[[#^s4|01");
    }
}
