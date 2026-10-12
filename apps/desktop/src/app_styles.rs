//! Writing styles per app: `writing_styles` holds the user's styles (a name and an instruction in
//! pt-BR, four of them ready), `app_styles` maps an app name to one of them. The instruction of
//! the app's style goes at the end of the dictation prompt; an app without a mapping keeps the
//! prompt it had before styles existed, destination hint included.

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::AppHandle;

use crate::settings::{self, AppSettings};

/// A writing style. `id` ties it to the apps that use it, so renaming keeps the mapping.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Type)]
pub struct WritingStyle {
    pub id: String,
    pub name: String,
    pub instruction: String,
}

/// An app, as `fala_inject::app_name_from_exe_path` names it, and the id of its style.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Type)]
pub struct AppStyle {
    pub app: String,
    pub style: String,
}

/// The four ready styles, one per destination of the prompt (chat, email, editor, prompt).
pub(crate) fn default_writing_styles() -> Vec<WritingStyle> {
    [
        (
            "message",
            "Mensagem",
            "Curto e informal, como numa conversa. Sem saudação nem assinatura. Mensagem de uma \
             linha pode terminar sem ponto final.",
        ),
        (
            "email",
            "E-mail",
            "Cordial e em parágrafos. Saudação e despedida só se eu ditar, cada uma na própria \
             linha; nunca invente uma.",
        ),
        (
            "document",
            "Documento",
            "Texto de documento: frases completas, parágrafos e listas como eu ditar, sem tom de \
             conversa.",
        ),
        (
            "code",
            "Código e terminal",
            "Literal, para um terminal ou um assistente de IA. Sem saudação e sem ponto final \
             forçado; comandos, caminhos, flags e identificadores exatamente como ditados.",
        ),
    ]
    .map(|(id, name, instruction)| WritingStyle {
        id: id.to_string(),
        name: name.to_string(),
        instruction: instruction.to_string(),
    })
    .to_vec()
}

/// Each mapped app with the instruction of its style, for `Postprocessor::with_app_styles`. A
/// mapping to a style that no longer exists is skipped.
pub(crate) fn instructions(settings: &AppSettings) -> Vec<(String, String)> {
    settings
        .app_styles
        .iter()
        .filter_map(|mapping| {
            settings
                .writing_styles
                .iter()
                .find(|style| style.id == mapping.style)
                .map(|style| (mapping.app.clone(), style.instruction.clone()))
        })
        .collect()
}

/// Trims names and instructions, drops styles without a name and gives an empty or repeated id a
/// fresh `style-N` that no style given uses, so a later style keeps its id and its apps.
fn normalize_styles(styles: Vec<WritingStyle>) -> Vec<WritingStyle> {
    let given: Vec<String> = styles.iter().map(|s| s.id.trim().to_string()).collect();
    let mut kept: Vec<WritingStyle> = Vec::new();
    for style in styles {
        let name = style.name.trim().to_string();
        if name.is_empty() {
            continue;
        }
        let mut id = style.id.trim().to_string();
        if id.is_empty() || kept.iter().any(|k| k.id == id) {
            id = (1..)
                .map(|n| format!("style-{n}"))
                .find(|candidate| {
                    !given.contains(candidate) && !kept.iter().any(|k| k.id == *candidate)
                })
                .unwrap_or_default();
        }
        kept.push(WritingStyle {
            id,
            name,
            instruction: style.instruction.trim().to_string(),
        });
    }
    kept
}

/// Reduces each app to its name and keeps one style per app, the last one given, at the place of
/// the first. A mapping to an unknown style is dropped.
fn normalize_app_styles(mappings: Vec<AppStyle>, styles: &[WritingStyle]) -> Vec<AppStyle> {
    let mut kept: Vec<AppStyle> = Vec::new();
    for mapping in mappings {
        let Some(app) = fala_inject::app_name_from_exe_path(mapping.app.trim()) else {
            continue;
        };
        if !styles.iter().any(|style| style.id == mapping.style) {
            continue;
        }
        match kept.iter_mut().find(|k| k.app == app) {
            Some(existing) => existing.style = mapping.style,
            None => kept.push(AppStyle {
                app,
                style: mapping.style,
            }),
        }
    }
    kept
}

/// Stores the styles and returns them as stored, with the mappings left once the styles that
/// are gone take their apps with them.
#[tauri::command]
#[specta::specta]
pub fn update_writing_styles(
    app: AppHandle,
    styles: Vec<WritingStyle>,
) -> Result<(Vec<WritingStyle>, Vec<AppStyle>), String> {
    let mut settings = settings::get_settings(&app);
    settings.writing_styles = normalize_styles(styles);
    settings.app_styles = normalize_app_styles(
        std::mem::take(&mut settings.app_styles),
        &settings.writing_styles,
    );
    let stored = (settings.writing_styles.clone(), settings.app_styles.clone());
    settings::write_settings(&app, settings);
    Ok(stored)
}

/// Stores the app -> style mapping and returns it as stored.
#[tauri::command]
#[specta::specta]
pub fn update_app_styles(
    app: AppHandle,
    app_styles: Vec<AppStyle>,
) -> Result<Vec<AppStyle>, String> {
    let mut settings = settings::get_settings(&app);
    settings.app_styles = normalize_app_styles(app_styles, &settings.writing_styles);
    let stored = settings.app_styles.clone();
    settings::write_settings(&app, settings);
    Ok(stored)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn style(id: &str, name: &str, instruction: &str) -> WritingStyle {
        WritingStyle {
            id: id.to_string(),
            name: name.to_string(),
            instruction: instruction.to_string(),
        }
    }

    fn mapping(app: &str, style: &str) -> AppStyle {
        AppStyle {
            app: app.to_string(),
            style: style.to_string(),
        }
    }

    #[test]
    fn four_ready_styles_with_portuguese_instructions() {
        let styles = default_writing_styles();
        let ids: Vec<&str> = styles.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, ["message", "email", "document", "code"]);
        assert!(styles
            .iter()
            .all(|s| !s.name.is_empty() && !s.instruction.is_empty()));
        assert_eq!(normalize_styles(styles.clone()), styles);
        let from_empty: AppSettings = serde_json::from_value(serde_json::json!({}))
            .expect("all AppSettings fields need serde defaults");
        for settings in [crate::settings::get_default_settings(), from_empty] {
            assert_eq!(settings.writing_styles, styles);
            assert!(settings.app_styles.is_empty());
            assert!(instructions(&settings).is_empty());
        }
    }

    #[test]
    fn styles_are_trimmed_named_and_unique() {
        let styles = normalize_styles(vec![
            style(" a ", " Formal ", "  Use você.  "),
            style("", "Sem id", "x"),
            style("a", "Id repetido", "y"),
            style("b", "   ", "sem nome"),
            style("style-1", "Já tinha", "z"),
        ]);
        assert_eq!(
            styles,
            [
                style("a", "Formal", "Use você."),
                style("style-2", "Sem id", "x"),
                style("style-3", "Id repetido", "y"),
                style("style-1", "Já tinha", "z"),
            ]
        );
    }

    #[test]
    fn mappings_use_the_app_name_and_one_style_per_app() {
        let styles = default_writing_styles();
        let mappings = normalize_app_styles(
            vec![
                mapping(r"C:\Program Files\Slack\Slack.exe", "message"),
                mapping("chrome", "email"),
                mapping("SLACK", "document"),
                mapping("code", "gone"),
                mapping("  ", "email"),
            ],
            &styles,
        );
        assert_eq!(
            mappings,
            [mapping("slack", "document"), mapping("chrome", "email")]
        );
    }

    #[test]
    fn instructions_follow_the_mapping() {
        let mut settings = crate::settings::get_default_settings();
        settings
            .writing_styles
            .push(style("mine", "Meu", "Assine Augusto."));
        settings.app_styles = vec![
            mapping("chrome", "email"),
            mapping("slack", "mine"),
            mapping("code", "gone"),
        ];
        let email = &default_writing_styles()[1].instruction;
        assert_eq!(
            instructions(&settings),
            [
                ("chrome".to_string(), email.clone()),
                ("slack".to_string(), "Assine Augusto.".to_string()),
            ]
        );
    }
}
