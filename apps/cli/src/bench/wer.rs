//! WER por palavra com a normalização fixa da door 3 do plano do `bench`.

use icu_normalizer::ComposingNormalizerBorrowed;

/// NFC → minúsculas → toda sequência que não seja letra ou dígito vira um espaço → split.
pub fn normalize(text: &str) -> Vec<String> {
    let nfc = ComposingNormalizerBorrowed::new_nfc().normalize(text);
    nfc.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Contagem de edições de um corte, ou a soma de vários (a linha `total`).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Score {
    pub sub: usize,
    pub del: usize,
    pub ins: usize,
    pub ref_words: usize,
}

impl Score {
    /// (S+D+I)/N em %, com 2 casas. Somar `Score`s antes de chamar dá o agregado Σ/Σ.
    pub fn wer_pct(&self) -> String {
        let edits = (self.sub + self.del + self.ins) as f64;
        format!("{:.2}", edits / self.ref_words as f64 * 100.0)
    }

    pub fn add(&mut self, other: &Score) {
        self.sub += other.sub;
        self.del += other.del;
        self.ins += other.ins;
        self.ref_words += other.ref_words;
    }
}

/// Menor edição por palavra (Levenshtein) entre referência e hipótese já normalizadas.
pub fn score(reference: &[String], hypothesis: &[String]) -> Score {
    let (n, m) = (reference.len(), hypothesis.len());
    let width = m + 1;
    let mut cost = vec![0usize; (n + 1) * width];
    for i in 0..=n {
        cost[i * width] = i;
    }
    for (j, cell) in cost.iter_mut().enumerate().take(width) {
        *cell = j;
    }
    for i in 1..=n {
        for j in 1..=m {
            let diag =
                cost[(i - 1) * width + j - 1] + usize::from(reference[i - 1] != hypothesis[j - 1]);
            let del = cost[(i - 1) * width + j] + 1;
            let ins = cost[i * width + j - 1] + 1;
            cost[i * width + j] = diag.min(del).min(ins);
        }
    }

    let mut s = Score {
        ref_words: n,
        ..Score::default()
    };
    let (mut i, mut j) = (n, m);
    while i > 0 || j > 0 {
        let here = cost[i * width + j];
        if i > 0 && j > 0 {
            let same = reference[i - 1] == hypothesis[j - 1];
            if here == cost[(i - 1) * width + j - 1] + usize::from(!same) {
                s.sub += usize::from(!same);
                i -= 1;
                j -= 1;
                continue;
            }
        }
        if i > 0 && here == cost[(i - 1) * width + j] + 1 {
            s.del += 1;
            i -= 1;
        } else {
            s.ins += 1;
            j -= 1;
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(text: &str) -> Vec<String> {
        normalize(text)
    }

    #[test]
    fn normalize_follows_door_3() {
        assert_eq!(
            normalize("A\u{301}  B-c'd, 2!"),
            vec!["á", "b", "c", "d", "2"]
        );
    }

    #[test]
    fn levenshtein_counts_sub_del_ins() {
        let sub = score(&words("a b c"), &words("a x c"));
        assert_eq!((sub.sub, sub.del, sub.ins, sub.ref_words), (1, 0, 0, 3));
        assert_eq!(sub.wer_pct(), "33.33");

        let del = score(&words("a b c"), &words("a c"));
        assert_eq!((del.sub, del.del, del.ins, del.ref_words), (0, 1, 0, 3));

        let ins = score(&words("a b"), &words("a b c"));
        assert_eq!((ins.sub, ins.del, ins.ins, ins.ref_words), (0, 0, 1, 2));
        assert_eq!(ins.wer_pct(), "50.00");
    }

    #[test]
    fn ola_mundo_is_zero() {
        let s = score(&words("Olá, mundo!"), &words("olá mundo"));
        assert_eq!(s.wer_pct(), "0.00");
    }

    #[test]
    fn guarda_chuva_counts_one_insertion() {
        let s = score(
            &words("o guarda-chuva ficou"),
            &words("o guarda chuva ficou aqui"),
        );
        assert_eq!((s.sub, s.del, s.ins, s.ref_words), (0, 0, 1, 4));
        assert_eq!(s.wer_pct(), "25.00");
    }
}
