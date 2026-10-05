//! As dependências de `fala-asr` são exatamente as da door 2 do plano `pipeline-headless`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

/// Nomes declarados numa seção `[...]` do manifesto, em ordem.
fn section(manifest: &str, header: &str) -> Vec<(String, String)> {
    manifest
        .lines()
        .skip_while(|l| l.trim() != header)
        .skip(1)
        .take_while(|l| !l.trim_start().starts_with('['))
        .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
        .filter_map(|l| {
            let (name, spec) = l.split_once('=')?;
            let name = name.trim().trim_end_matches(".workspace").to_owned();
            Some((name, spec.trim().to_owned()))
        })
        .collect()
}

#[test]
fn dependencies_match_door_2() {
    let manifest = include_str!("../Cargo.toml");
    let deps = section(manifest, "[dependencies]");
    let mut names: Vec<&str> = deps.iter().map(|(n, _)| n.as_str()).collect();
    names.sort_unstable();
    assert_eq!(names, ["fala-core", "thiserror", "transcribe-rs"]);
    let spec = |name: &str| deps.iter().find(|(n, _)| n == name).unwrap().1.clone();
    assert_eq!(
        spec("transcribe-rs"),
        "{ version = \"0.3.8\", features = [\"onnx\"] }"
    );
    let dev = section(manifest, "[dev-dependencies]");
    assert_eq!(dev, [("hound".to_owned(), "\"3.5.1\"".to_owned())]);
    assert!(!manifest.contains("sherpa"));
    assert!(!manifest.contains("tauri"));
}
