//! TypeScript bindings for the wire types, checked into each client.
//!
//! The test fails while a client's checked-in bindings differ from what the Rust
//! types generate, and rewrites them so that committing the result fixes it.

use crate::game::GameUpdate;
use crate::question::{Color, QuestionType};
use crate::server::{
    CheckSessionsRequest, CheckSessionsResponse, ClientMessage, CreateLobbyRequest,
    CreateLobbyResponse, ErrorResponse, GetStoredDataRequest, JoinLobbyRequest, JoinLobbyResponse,
    ListSetsResponse, SetStoredDataRequest, StoredDataResponse, UploadCharacterImageResponse,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use ts_rs::{Config, ExportError, TS};

type Export = fn(&Config) -> Result<(), ExportError>;

const FRONTEND: &[Export] = &[
    GameUpdate::export_all,
    ClientMessage::export_all,
    ListSetsResponse::export_all,
    CreateLobbyRequest::export_all,
    CreateLobbyResponse::export_all,
    JoinLobbyRequest::export_all,
    JoinLobbyResponse::export_all,
    CheckSessionsRequest::export_all,
    CheckSessionsResponse::export_all,
    ErrorResponse::export_all,
];

const ADMIN_PANEL: &[Export] = &[
    StoredDataResponse::export_all,
    GetStoredDataRequest::export_all,
    SetStoredDataRequest::export_all,
    UploadCharacterImageResponse::export_all,
    ErrorResponse::export_all,
    Color::export_all,
    QuestionType::export_all,
];

fn config(out_dir: &Path) -> Config {
    Config::new()
        .with_large_int("number")
        .with_import_extension(Some("ts"))
        .with_out_dir(out_dir)
}

fn read_dir(dir: &Path) -> BTreeMap<String, String> {
    let Ok(entries) = fs::read_dir(dir) else {
        return BTreeMap::new();
    };
    entries
        .map(|entry| {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            (name, fs::read_to_string(&path).unwrap())
        })
        .collect()
}

fn check_bindings(client: &str, exports: &[Export]) {
    let generated_dir = tempfile::tempdir().unwrap();
    let cfg = config(generated_dir.path());
    for export in exports {
        export(&cfg).unwrap();
    }
    let generated = read_dir(generated_dir.path());

    let checked_in_dir: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "..",
        client,
        "src/lib/generated",
    ]
    .iter()
    .collect();
    if read_dir(&checked_in_dir) == generated {
        return;
    }

    if checked_in_dir.exists() {
        fs::remove_dir_all(&checked_in_dir).unwrap();
    }
    fs::create_dir_all(&checked_in_dir).unwrap();
    for (name, contents) in &generated {
        fs::write(checked_in_dir.join(name), contents).unwrap();
    }
    panic!(
        "{client}/src/lib/generated was out of date and has been regenerated; review and commit it"
    );
}

#[test]
fn frontend_bindings_are_current() {
    check_bindings("frontend", FRONTEND);
}

#[test]
fn admin_panel_bindings_are_current() {
    check_bindings("admin_panel", ADMIN_PANEL);
}
