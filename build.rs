#![allow(unused)]

use std::{
    collections::HashMap,
    env, fmt,
    fs::{self, File},
    io::{self, Write},
    path::{Path, Prefix},
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

#[derive(Debug, thiserror::Error)]
enum MainError {
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("Missing nvim stdout")]
    NvimStdout,
    #[error("{0}")]
    Rmp(#[from] rmp_serde::decode::Error),
}

fn main() -> Result<(), MainError> {
    let mut nvim = Command::new("nvim")
        .arg("--api-info")
        .stdout(Stdio::piped())
        .spawn()?;
    let mut stdout = nvim.stdout.take().ok_or(MainError::NvimStdout)?;
    let root: Root = from_read(stdout)?;
    // warn!("{:?}", root.ui_options);

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir).join("nvim.rs");
    let mut out_file = File::create(out_path)?;
    write_error_types(&mut out_file, &root.error_types)?;
    write_version(&mut out_file, &root.version)?;
    write_types(&mut out_file, &root.types)?;
    write_ui_options(&mut out_file, &root.ui_options)?;
    println!("cargo:rerun-if-changed=build.rs");
    Ok(())
}

fn write_ui_options(dst: &mut impl Write, ui_options: &[String]) -> io::Result<()> {
    let enum_names: Vec<_> = ui_options
        .iter()
        .map(|option| {
            option
                .split('_')
                .flat_map(|part| {
                    let mut chars = part.chars();
                    let first = chars.next().map(|c| c.to_uppercase());
                    first.into_iter().flatten().chain(chars)
                })
                .collect::<String>()
        })
        .collect();

    write!(dst, "pub enum UiOption {{\n")?;
    for enum_name in enum_names.iter() {
        write!(dst, "{enum_name},\n")?;
    }
    write!(
        dst,
        "}}

        impl From<UiOption> for &str {{
            fn from(value: UiOption) -> Self {{
                match value {{
        "
    )?;
    for (enum_name, name) in enum_names.iter().zip(ui_options.iter()) {
        write!(dst, "UiOption::{enum_name} => \"{name}\",")?;
    }
    write!(dst, "}} }} }}")?;
    Ok(())
}

fn write_types(dst: &mut impl Write, types: &Types) -> io::Result<()> {
    for (name, t) in types.into_iter() {
        write!(
            dst,
            "pub struct {name} {{
                pub data: i64,
            }}

            impl {name} {{
                pub const ID: i64 = {};
            }}",
            t.id
        )?;
    }
    Ok(())
}

fn write_version(dst: &mut impl Write, version: &Version) -> io::Result<()> {
    write!(
        dst,
        "pub struct Version {{
            pub api_compatible: i64,
            pub api_level: i64,
            pub api_prerelease: bool,
            pub major: i64,
            pub minor: i64,
            pub patch: i64,
            pub prerelease: bool,
        }}
        impl Version {{
            pub const CURRENT: Self = Self {{
                api_compatible: {},
                api_level: {},
                api_prerelease: {},
                major: {},
                minor: {},
                patch: {},
                prerelease: {},
            }};
        }}",
        version.api_compatible,
        version.api_level,
        version.api_prerelease,
        version.major,
        version.minor,
        version.patch,
        version.prerelease
    )?;
    Ok(())
}

fn write_error_types(dst: &mut impl Write, error_types: &ErrorTypes) -> io::Result<()> {
    write!(dst, "pub enum Error {{\n")?;
    for (name, t) in error_types.iter() {
        write!(dst, "{name} = {},", t.id)?;
    }
    write!(dst, "}}\n")?;
    Ok(())
}

type ErrorTypes = HashMap<String, ErrorType>;
type Types = HashMap<String, Type>;

#[derive(Debug, Deserialize)]
struct Root {
    version: Version,
    error_types: ErrorTypes,
    types: Types,
    functions: Vec<Function>,
    ui_options: Vec<String>,
    ui_events: Vec<UiEvent>,
}

#[derive(Debug, Deserialize)]
struct Version {
    api_compatible: i64,
    api_level: i64,
    api_prerelease: bool,
    major: i64,
    minor: i64,
    patch: i64,
    prerelease: bool,
}

#[derive(Debug, Deserialize)]
struct ErrorType {
    id: i64,
}

#[derive(Debug, Deserialize)]
struct Type {
    id: i64,
    prefix: String,
}

#[derive(Debug, Deserialize)]
struct Function {
    method: bool,
    name: String,
    parameters: Vec<Parameter>,
    return_type: TypeName,
    since: i64,
    deprecated_since: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct UiEvent {
    name: String,
    parameters: Vec<Parameter>,
    since: i64,
}

#[derive(Debug)]
struct Parameter {
    type_name: TypeName,
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

#[derive(Debug)]
enum TypeName {
    FixedArray { size: i64, type_name: String },
    DynamicArray(String),
    Other(String),
}

impl<'de> Deserialize<'de> for TypeName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_string(TypeNameVisitor)
    }
}

struct TypeNameVisitor;

impl<'de> Visitor<'de> for TypeNameVisitor {
    type Value = TypeName;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        const PREFIX: &str = "ArrayOf(";
        const SEP: &str = ", ";
        let is_array = v
            .chars()
            .zip(PREFIX.chars())
            .all(|(actual, expected)| actual == expected)
            && v.len() > PREFIX.len();

        if is_array {
            let type_name: String = v
                .chars()
                .skip(PREFIX.chars().count())
                .take_while(|c| c.is_ascii_alphabetic())
                .collect();

            let is_fixed = v
                .chars()
                .skip(PREFIX.chars().count() + type_name.chars().count())
                .zip(SEP.chars())
                .all(|(actual, expected)| actual == expected);
            if is_fixed {
                let size: String = v
                    .chars()
                    .skip(PREFIX.chars().count() + type_name.chars().count() + SEP.chars().count())
                    .take_while(|c| c.is_ascii_digit())
                    .collect();
                let size: i64 = size.parse().unwrap();
                Ok(TypeName::FixedArray { size, type_name })
            } else {
                Ok(TypeName::DynamicArray(type_name))
            }
        } else {
            Ok(TypeName::Other(v.to_string()))
        }
    }
}
