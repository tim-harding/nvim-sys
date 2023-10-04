use std::{
    env, fs,
    path::Path,
    process::{Command, Stdio},
};

use rmp_serde::from_read;
use serde::Deserialize;

macro_rules! warn {
    ($($tokens: tt)*) => {
        println!("cargo:warning={}", format!($($tokens)*))
    }
}

fn main() {
    let mut nvim = Command::new("nvim")
        .arg("--api-info")
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdout = nvim.stdout.take().unwrap();
    let root: Root = from_read(stdout).unwrap();
    warn!("{:?}", root);

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("nvim.rs");
    fs::write(
        &dest_path,
        "pub fn message() -> &'static str {
            \"Hello, World!\"
        }
        ",
    )
    .unwrap();
    println!("cargo:rerun-if-changed=build.rs");
}

#[derive(Debug, Deserialize)]
struct Root {
    version: Version,
}

#[derive(Debug, Deserialize)]
struct Version {
    api_compatible: u64,
    api_level: u64,
    api_prerelease: bool,
    major: u64,
    minor: u64,
    patch: u64,
    prerelease: bool,
}
