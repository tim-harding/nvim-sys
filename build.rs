#![allow(unused)]

use std::{
    collections::HashMap,
    env, fmt,
    fs::{self, File},
    io::{self, BufWriter, Write},
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

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir).join("nvim.rs");
    let out_file = File::create(out_path)?;
    let mut w = BufWriter::new(out_file);
    write_version(&mut w, &root.version)?;
    write_functions(&mut w, &root.functions)?;
    println!("cargo:rerun-if-changed=build.rs");
    Ok(())
}

fn write_functions(dst: &mut impl Write, functions: &[Function]) -> io::Result<()> {
    // TODO: Method, since, deprecated since
    write!(
        dst,
        "pub mod functions {{
        use super::{{Buffer, Window, Tabpage, Array, BasicType, Dictionary, Neovim}};"
    )?;
    for function in functions.iter() {
        if function.parameters.iter().any(|p| match &p.type_name {
            TypeName::Other(type_name) => type_name.as_str() == "LuaRef",
            _ => false,
        }) {
            continue;
        }

        write!(
            dst,
            "#[allow(unused)] pub async fn {}(neovim: &mut impl Neovim, ",
            function.name
        )?;
        for parameter in function.parameters.iter() {
            let name = match parameter.name.as_str() {
                "fn" => "r#fn",
                "type" => "r#type",
                other => other,
            };
            write!(dst, "{name}: ")?;
            match &parameter.type_name {
                TypeName::FixedArray { size, type_name } => {
                    write!(dst, "[{}; {size}]", map_parameter_type_name(type_name))?
                }
                TypeName::DynamicArray(type_name) => write!(
                    dst,
                    "impl Iterator<Item = {}>",
                    map_parameter_type_name(type_name)
                )?,
                TypeName::Other(type_name) => {
                    write!(dst, "{}", map_parameter_type_name(type_name))?
                }
            }
            write!(dst, ", ")?;
        }
        write!(dst, ") ");
        match &function.return_type {
            TypeName::FixedArray { size, type_name } => write!(
                dst,
                "-> [{}; {size}]",
                map_return_type_name(type_name, &function.name)
            )?,
            TypeName::DynamicArray(type_name) => write!(
                dst,
                "-> Vec<{}>",
                map_return_type_name(type_name, &function.name)
            )?,
            TypeName::Other(type_name) => match type_name.as_str() {
                "void" => {}
                _ => write!(
                    dst,
                    "-> {}",
                    map_return_type_name(type_name, &function.name)
                )?,
            },
        }
        write!(
            dst,
            "{{ 
                todo!()
            }}\n"
        )?;
    }
    write!(dst, "}}")?;
    Ok(())
}

fn map_parameter_type_name(type_name: &str) -> &str {
    match type_name {
        "Boolean" => "bool",
        "Integer" => "i64",
        "Float" => "f64",
        "String" => "&str",
        "Object" => "BasicType",
        _ => type_name,
    }
}

fn map_return_type_name<'a>(type_name: &'a str, function_name: &'a str) -> &'a str {
    match type_name {
        "Boolean" => "bool",
        "Integer" => "i64",
        "Float" => "f64",
        "Object" => {
            if function_name.chars().take(6).eq("window".chars()) {
                "Window"
            } else if function_name.chars().take(7).eq("tabpage".chars()) {
                "Tabpage"
            } else if function_name.chars().take(6).eq("buffer".chars()) {
                "Buffer"
            } else {
                "BasicType"
            }
        }
        _ => type_name,
    }
}

fn snake_to_camel(s: &str) -> String {
    s.split('_')
        .flat_map(|part| {
            let mut chars = part.chars();
            let first = chars.next().map(|c| c.to_uppercase());
            first.into_iter().flatten().chain(chars)
        })
        .collect()
}

fn write_version(dst: &mut impl Write, version: &Version) -> io::Result<()> {
    write!(
        dst,
        "
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
