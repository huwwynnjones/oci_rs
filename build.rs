extern crate build_helper;

use build_helper::{rustc, target, LibKind};

fn main() {
    let lib_name = match target::triple().os() {
        "windows" => "oci",
        _ => "clntsh"
    };
    rustc::link_lib(Some(LibKind::DyLib), lib_name);
}
