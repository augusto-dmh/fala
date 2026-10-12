/// Um snippet: quando o ditado tem o gatilho, o texto final tem a expansão, que o LLM nunca vê.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snippet {
    trigger: String,
    expansion: String,
}

impl Snippet {
    /// `None` se o gatilho tem menos de duas palavras (um gatilho de uma palavra seria dito sem
    /// querer) ou se a expansão fica vazia. Espaços seguidos no gatilho viram um; a expansão só
    /// perde os espaços das pontas e guarda as quebras de linha do meio.
    pub fn new(trigger: impl AsRef<str>, expansion: impl AsRef<str>) -> Option<Self> {
        let words: Vec<&str> = trigger.as_ref().split_whitespace().collect();
        let expansion = expansion.as_ref().trim().to_string();
        (words.len() >= 2 && !expansion.is_empty()).then(|| Self {
            trigger: words.join(" "),
            expansion,
        })
    }

    pub fn trigger(&self) -> &str {
        &self.trigger
    }

    pub fn expansion(&self) -> &str {
        &self.expansion
    }
}

/// O instante local em que as variáveis de um snippet são resolvidas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Moment {
    pub year: i32,
    /// 1 a 12.
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
}

const MONTHS_PT_BR: [&str; 12] = [
    "janeiro",
    "fevereiro",
    "março",
    "abril",
    "maio",
    "junho",
    "julho",
    "agosto",
    "setembro",
    "outubro",
    "novembro",
    "dezembro",
];

/// Resolve `{data}` (`02/10/2026`), `{data_extenso}` (`2 de outubro de 2026`) e `{hora}`
/// (`14:05`); o resto do texto fica como está.
pub fn render_snippet(template: &str, at: Moment) -> String {
    let month = at
        .month
        .checked_sub(1)
        .and_then(|i| MONTHS_PT_BR.get(i as usize))
        .copied()
        .unwrap_or_default();
    template
        .replace(
            "{data}",
            &format!("{:02}/{:02}/{}", at.day, at.month, at.year),
        )
        .replace(
            "{data_extenso}",
            &format!("{} de {month} de {}", at.day, at.year),
        )
        .replace("{hora}", &format!("{:02}:{:02}", at.hour, at.minute))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trigger_needs_two_words() {
        assert!(Snippet::new("assinatura", "Augusto").is_none());
        assert!(Snippet::new("minha assinatura", "  \n ").is_none());
        let snippet = Snippet::new("  minha   assinatura ", "\nAugusto\nfala.dev\n").unwrap();
        assert_eq!(snippet.trigger(), "minha assinatura");
        assert_eq!(snippet.expansion(), "Augusto\nfala.dev");
    }

    #[test]
    fn renders_the_variables() {
        let at = Moment {
            year: 2026,
            month: 10,
            day: 2,
            hour: 14,
            minute: 5,
        };
        assert_eq!(
            render_snippet("{data} · {data_extenso} · {hora} · {outra}", at),
            "02/10/2026 · 2 de outubro de 2026 · 14:05 · {outra}"
        );
        let march = Moment { month: 3, ..at };
        assert_eq!(
            render_snippet("{data_extenso}", march),
            "2 de março de 2026"
        );
    }
}
