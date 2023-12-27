use std::collections::HashMap;

pub type Array = Vec<BasicType>;
pub type Dictionary = HashMap<BasicType, BasicType>;

pub enum BasicType {
    Nil,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Array(Array),
    Dictionary(Dictionary),
    Object(SpecialType),
}

pub enum SpecialType {
    Buffer(Buffer),
    Window(Window),
    Tabpage(Tabpage),
}

pub struct Buffer {
    pub bufnr: i64,
}

pub struct Window {
    pub window_id: i64,
}

pub struct Tabpage {
    pub handle: i64,
}

pub struct Version {
    pub api_compatible: i64,
    pub api_level: i64,
    pub api_prerelease: bool,
    pub major: i64,
    pub minor: i64,
    pub patch: i64,
    pub prerelease: bool,
}

include!(concat!(env!("OUT_DIR"), "/nvim.rs"));
