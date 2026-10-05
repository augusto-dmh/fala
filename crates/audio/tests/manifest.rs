//! As dependências de `fala-audio` são exatamente as da door 1 do plano `pipeline-headless`, mais o
//! `hound` da door 4 do plano `meeting-recorder`.

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
fn dependencies_match_door_1() {
    let manifest = include_str!("../Cargo.toml");
    let deps = section(manifest, "[dependencies]");
    let mut names: Vec<&str> = deps.iter().map(|(n, _)| n.as_str()).collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "cpal",
            "fala-core",
            "hound",
            "log",
            "rtrb",
            "rubato",
            "thiserror",
            "vad-rs"
        ]
    );
    let spec = |name: &str| deps.iter().find(|(n, _)| n == name).unwrap().1.clone();
    assert_eq!(spec("cpal"), "\"0.16.0\"");
    assert_eq!(spec("rtrb"), "\"0.4.0\"");
    assert_eq!(spec("rubato"), "\"0.16.2\"");
    assert_eq!(spec("log"), "\"0.4\"");
    assert_eq!(
        spec("vad-rs"),
        "{ git = \"https://github.com/cjpais/vad-rs\", default-features = false }"
    );
    assert_eq!(spec("hound"), "\"3.5.1\"");
    assert!(!manifest.contains("[dev-dependencies]"));
    assert!(!manifest.contains("sherpa"));
    assert!(!manifest.contains("tauri"));
}
