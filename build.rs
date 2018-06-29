extern crate build_helper;

use build_helper::{host, rustc, target, LibKind, SearchKind};

use std::env;
use std::fs::ReadDir;
use std::path::PathBuf;

fn main() {
    let lib_name = match target::triple().os() {
        "windows" => "oci",
        _ => "clntsh",
    };
    rustc::link_lib(Some(LibKind::DyLib), lib_name);

    let host = host();
    match (host.os(), host.env()) {
        ("windows", Some("gnu")) => if let Some(path) = find_dll(lib_name) {
            rustc::link_search(Some(SearchKind::Native), path);
        },
        _ => (),
    }
}

fn find_dll(dll_name: &str) -> Option<PathBuf> {
    assert_eq!(host().os(), "windows");
    let contains_dll = |mut contained_files: ReadDir| {
        contained_files.any(|maybe_entry| {
            maybe_entry
                .ok()
                .and_then(|entry| entry.file_name().into_string().ok())
                .map(|file_name| file_name.to_lowercase() == dll_name.to_string() + ".dll")
                .unwrap_or(false)
        })
    };
    env::var_os("PATH").and_then(|paths| {
        env::split_paths(&paths)
            .filter(|path| path.read_dir().map(&contains_dll).unwrap_or(false))
            .next()
    })
}
