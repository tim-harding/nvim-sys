#![allow(unused)]

use std::{
    collections::HashMap,
    env, fmt, fs,
    path::Path,
    process::{Command, Stdio},
};

use rmp_serde::from_read;
use serde::{
    de::{self, SeqAccess, Visitor},
    Deserialize,
};

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
    error_types: HashMap<String, ErrorType>,
    types: HashMap<String, Type>,
    functions: Vec<Function>,
    ui_options: Vec<String>,
    ui_events: Vec<UiEvent>,
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

#[derive(Debug, Deserialize)]
struct ErrorType {
    id: u64,
}

#[derive(Debug, Deserialize)]
struct Type {
    id: u64,
    prefix: String,
}

#[derive(Debug, Deserialize)]
struct Function {
    method: bool,
    name: String,
    parameters: Vec<Parameter>,
    return_type: String,
    since: u64,
    deprecated_since: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct UiEvent {
    name: String,
    parameters: Vec<Parameter>,
    since: u64,
}

#[derive(Debug)]
struct Parameter {
    type_name: String,
    name: String,
}

impl<'de> Deserialize<'de> for Parameter {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_tuple(2, ParameterVisitor)
    }
}

struct ParameterVisitor;

impl<'de> Visitor<'de> for ParameterVisitor {
    type Value = Parameter;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a fixed-length array of size 2")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Parameter, A::Error>
    where
        A: SeqAccess<'de>,
    {
        Ok(Parameter {
            type_name: seq
                .next_element()?
                .ok_or_else(|| de::Error::invalid_length(0, &self))?,
            name: seq
                .next_element()?
                .ok_or_else(|| de::Error::invalid_length(0, &self))?,
        })
    }
}
