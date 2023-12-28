use rmp::decode::ValueReadError;
use std::{
    collections::HashMap,
    fmt::Write,
    io::{self, Read},
    string::FromUtf8Error,
};

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

pub enum BasicTypeKind {
    Nil,
    Boolean,
    Integer,
    Float,
    String,
    Array,
    Dictionary,
    Object,
}

trait FromMsgpack {
    fn from_msgpack(r: &mut impl Read) -> Result<Self, FromMsgpackError>;
}

#[derive(Debug, thiserror::Error)]
pub enum FromMsgpackError {
    #[error("{0}")]
    Rmp(#[from] ValueReadError<io::Error>),
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("{0}")]
    String(#[from] FromUtf8Error),
    #[error("Unexpected MsgPack type")]
    Marker {
        expected: BasicTypeKind,
        actual: rmp::Marker,
    },
}

impl FromMsgpack for bool {
    fn from_msgpack(r: &mut impl Read) -> Result<Self, FromMsgpackError> {
        match rmp::decode::read_marker(r)? {
            rmp::Marker::True => Ok(true),
            rmp::Marker::False => Ok(false),
            marker => Err(FromMsgpackError::Marker {
                expected: BasicTypeKind::Boolean,
                actual: marker,
            }),
        }
    }
}

impl FromMsgpack for i64 {
    fn from_msgpack(r: &mut impl Read) -> Result<Self, ValueReadError> {
        match rmp::decode::read_marker(r)? {
            rmp::Marker::U8 => Ok(rmp::decode::read_u8(r)? as i64),
            rmp::Marker::U16 => Ok(rmp::decode::read_u16(r)? as i64),
            rmp::Marker::U32 => Ok(rmp::decode::read_u32(r)? as i64),
            rmp::Marker::U64 => Ok(rmp::decode::read_u64(r)? as i64),
            rmp::Marker::I8 => Ok(rmp::decode::read_i8(r)? as i64),
            rmp::Marker::I16 => Ok(rmp::decode::read_i16(r)? as i64),
            rmp::Marker::I32 => Ok(rmp::decode::read_i32(r)? as i64),
            rmp::Marker::I64 => Ok(rmp::decode::read_i64(r)? as i64),
            marker => Err(FromMsgpackError::Marker {
                expected: BasicTypeKind::Integer,
                actual: marker,
            }),
        }
    }
}

impl FromMsgpack for f64 {
    fn from_msgpack(r: &mut impl Read) -> Result<Self, FromMsgpackError> {
        match rmp::decode::read_marker(r)? {
            rmp::Marker::F32 => Ok(rmp::decode::read_f32(r)? as f64),
            rmp::Marker::F64 => Ok(rmp::decode::read_f64(r)? as f64),
            marker => Err(FromMsgpackError::Marker {
                expected: BasicTypeKind::Float,
                actual: marker,
            }),
        }
    }
}

impl FromMsgpack for String {
    fn from_msgpack(r: &mut impl Read) -> Result<Self, FromMsgpackError> {
        let len = match rmp::decode::read_marker(r)? {
            rmp::Marker::FixStr(len) => len as usize,
            rmp::Marker::Str8 => read_u8(r)?,
            rmp::Marker::Str16 => read_u16(r)?,
            rmp::Marker::Str32 => read_u32(r)?,
            marker => Err(FromMsgpackError::Marker {
                expected: BasicTypeKind::String,
                actual: marker,
            }),
        };

        let mut buf = vec![0; len];
        r.read_exact(buf.as_mut_slice())?;
        Ok(String::from_utf8(buf)?)
    }
}

impl<T> FromMsgpack for Vec<T>
where
    T: FromMsgpack,
{
    fn from_msgpack(r: &mut impl Read) -> Result<Self, FromMsgpackError> {
        let len = match rmp::decode::read_marker(r)? {
            rmp::Marker::FixArray(len) => len as usize,
            rmp::Marker::Array16 => read_u16(r)? as usize,
            rmp::Marker::Array32 => read_u32(r)? as usize,
            marker => Err(FromMsgpackError::Marker {
                expected: BasicTypeKind::Array,
                actual: marker,
            }),
        };

        (0..len).map(|_| T::from_msgpack(r)).collect()
    }
}

impl<K, V> FromMsgpack for HashMap<K, V>
where
    K: FromMsgpack,
    V: FromMsgpack,
{
    fn from_msgpack(r: &mut impl Read) -> Result<Self, FromMsgpackError> {
        let len = match rmp::decode::read_marker(r)? {
            rmp::Marker::FixMap(len) => len as usize,
            rmp::Marker::Map16 => read_u16(r)? as usize,
            rmp::Marker::Map32 => read_u32(r)? as usize,
            marker => Err(FromMsgpackError::Marker {
                expected: BasicTypeKind::Dictionary,
                actual: marker,
            }),
        };

        (0..len)
            .map(|_| (K::from_msgpack(r)?, V::from_msgpack(r)?))
            .collect()
    }
}

impl FromMsgpack for Buffer {
    fn from_msgpack(r: &mut impl Read) -> Result<Self, FromMsgpackError> {
        match rmp::decode::read_marker(r)? {
            rmp::Marker::FixExt1 => todo!(),
            rmp::Marker::FixExt2 => todo!(),
            rmp::Marker::FixExt4 => todo!(),
            rmp::Marker::FixExt8 => todo!(),
            rmp::Marker::FixExt16 => todo!(),
            rmp::Marker::Ext8 => todo!(),
            rmp::Marker::Ext16 => todo!(),
            rmp::Marker::Ext32 => todo!(),
            marker => Err(FromMsgpackError::Marker {
                expected: BasicTypeKind::Object,
                actual: marker,
            }),
        }
    }
}

fn read_u8(r: &mut impl Read) -> io::Result<u8> {
    let mut buf = [0; 1];
    r.read_exact(&mut buf)?;
    Ok(buf[0])
}

fn read_u16(r: &mut impl Read) -> io::Result<u16> {
    let mut buf = [0; 2];
    r.read_exact(&mut buf)?;
    Ok(u16::from_be_bytes(buf))
}

fn read_u32(r: &mut impl Read) -> io::Result<u32> {
    let mut buf = [0; 4];
    r.read_exact(&mut buf)?;
    Ok(u32::from_be_bytes(buf))
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

pub trait Neovim {
    type R: Read;
    type W: Write;

    fn call<Return: FromMsgpack>(
        &mut self,
        method: &str,
        argument_writer: impl Fn(&mut Self::W),
    ) -> Return;
}

include!(concat!(env!("OUT_DIR"), "/nvim.rs"));
