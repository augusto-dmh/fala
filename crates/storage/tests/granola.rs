//! O export do Granola lido do disco.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::io::Write;

use common::env;
use fala_storage::read_granola_export;

const HEADER: &str =
    "document_id,document_title,workspace_name,document_created,summary,notes,transcript\n";

fn csv(title: &str) -> String {
    format!(
        "{HEADER}g-1,{title},Acme,2025-03-14 10:30:00 BST,\"### Próximos passos\n- Ana manda o contrato\",\"pauta: preço\",\"Me: Bom dia.\nAna: Bom dia, vamos ao preço?\"\n\
         g-2,,Acme,2025-03-15T09:00:00Z,,,\n"
    )
}

#[test]
fn a_zip_named_csv_is_read_by_its_content() {
    let env = env("granola-zip");
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    writer
        .start_file("Granola export/Acme.csv", options)
        .unwrap();
    writer.write_all(csv("Do zip").as_bytes()).unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    let path = env.dir.join("granola-export.csv");
    std::fs::write(&path, bytes).unwrap();
    let export = read_granola_export(&path).unwrap();
    assert_eq!(export.meetings.len(), 2);
    assert_eq!(export.meetings[0].title, "Do zip");
}
