use rmp::{
    decode::{MarkerReadError, ValueReadError},
    encode::ValueWriteError,
};
use std::{
    collections::HashMap,
    io::{self, Read, Write},
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

#[derive(Debug)]
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

pub trait ToMsgpack {
    fn to_msgpack(self, w: &mut impl Write) -> Result<(), ToMsgpackError>;
}

#[derive(Debug, thiserror::Error)]
pub enum ToMsgpackError {
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("{0}")]
    Rmp(#[from] ValueWriteError),
}

impl ToMsgpack for bool {
    fn to_msgpack(self, w: &mut impl Write) -> Result<(), ToMsgpackError> {
        rmp::encode::write_bool(w, self)?;
        Ok(())
    }
}

impl ToMsgpack for i64 {
    fn to_msgpack(self, w: &mut impl Write) -> Result<(), ToMsgpackError> {
        rmp::encode::write_sint(w, self)?;
        Ok(())
    }
}

impl ToMsgpack for f64 {
    fn to_msgpack(self, w: &mut impl Write) -> Result<(), ToMsgpackError> {
        rmp::encode::write_f64(w, self)?;
        Ok(())
    }
}

impl ToMsgpack for &str {
    fn to_msgpack(self, w: &mut impl Write) -> Result<(), ToMsgpackError> {
        rmp::encode::write_str(w, self)?;
        Ok(())
    }
}

pub struct MsgpackArrayWriter<T, I>
where
    T: ToMsgpack,
    I: Iterator<Item = T>,
{
    len: u32,
    iter: I,
}

impl<T, I> ToMsgpack for MsgpackArrayWriter<T, I>
where
    T: ToMsgpack,
    I: Iterator<Item = T>,
{
    fn to_msgpack(self, w: &mut impl Write) -> Result<(), ToMsgpackError> {
        rmp::encode::write_array_len(w, self.len)?;
        for t in self.iter {
            t.to_msgpack(w)?;
        }
        Ok(())
    }
}

pub struct MsgpackDictionaryWriter<K, V, I>
where
    K: ToMsgpack,
    V: ToMsgpack,
    I: Iterator<Item = (K, V)>,
{
    len: u32,
    iter: I,
}

impl<K, V, I> ToMsgpack for MsgpackDictionaryWriter<K, V, I>
where
    K: ToMsgpack,
    V: ToMsgpack,
    I: Iterator<Item = (K, V)>,
{
    fn to_msgpack(self, w: &mut impl Write) -> Result<(), ToMsgpackError> {
        rmp::encode::write_map_len(w, self.len)?;
        for (k, v) in self.iter {
            k.to_msgpack(w)?;
            v.to_msgpack(w)?;
        }
        Ok(())
    }
}

pub trait FromMsgpack: Sized {
    fn from_msgpack(r: &mut impl Read) -> Result<Self, FromMsgpackError>;
}

#[derive(Debug, thiserror::Error)]
pub enum FromMsgpackError {
    #[error("{0}")]
    ValueRead(#[from] ValueReadError<io::Error>),
    #[error("Failed to read marker: {0}")]
    MarkerRead(io::Error),
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

impl From<MarkerReadError<io::Error>> for FromMsgpackError {
    fn from(value: MarkerReadError<io::Error>) -> Self {
        Self::MarkerRead(value.0)
    }
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
    fn from_msgpack(r: &mut impl Read) -> Result<Self, FromMsgpackError> {
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
            rmp::Marker::Str8 => read_u8(r)? as usize,
            rmp::Marker::Str16 => read_u16(r)? as usize,
            rmp::Marker::Str32 => read_u32(r)? as usize,
            marker => {
                return Err(FromMsgpackError::Marker {
                    expected: BasicTypeKind::String,
                    actual: marker,
                })
            }
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
            marker => {
                return Err(FromMsgpackError::Marker {
                    expected: BasicTypeKind::Array,
                    actual: marker,
                })
            }
        };

        (0..len).map(|_| T::from_msgpack(r)).collect()
    }
}

impl<K, V> FromMsgpack for HashMap<K, V>
where
    K: FromMsgpack + Eq + std::hash::Hash,
    V: FromMsgpack,
{
    fn from_msgpack(r: &mut impl Read) -> Result<Self, FromMsgpackError> {
        let len = match rmp::decode::read_marker(r)? {
            rmp::Marker::FixMap(len) => len as usize,
            rmp::Marker::Map16 => read_u16(r)? as usize,
            rmp::Marker::Map32 => read_u32(r)? as usize,
            marker => {
                return Err(FromMsgpackError::Marker {
                    expected: BasicTypeKind::Dictionary,
                    actual: marker,
                })
            }
        };

        (0..len)
            .map(|_| -> Result<_, _> { Ok((K::from_msgpack(r)?, V::from_msgpack(r)?)) })
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

impl Buffer {
    pub const TYPE_ID: i8 = 0;
}

impl ToMsgpack for Buffer {
    fn to_msgpack(self, w: &mut impl Write) -> Result<(), ToMsgpackError> {
        write_special_type(w, Self::TYPE_ID, self.bufnr)?;
        Ok(())
    }
}

pub struct Window {
    pub window_id: i64,
}

impl Window {
    pub const TYPE_ID: i8 = 1;
}

impl ToMsgpack for Window {
    fn to_msgpack(self, w: &mut impl Write) -> Result<(), ToMsgpackError> {
        write_special_type(w, Self::TYPE_ID, self.window_id)?;
        Ok(())
    }
}

pub struct Tabpage {
    pub handle: i64,
}

impl Tabpage {
    pub const TYPE_ID: i8 = 2;
}

impl ToMsgpack for Tabpage {
    fn to_msgpack(self, w: &mut impl Write) -> Result<(), ToMsgpackError> {
        write_special_type(w, Self::TYPE_ID, self.handle)?;
        Ok(())
    }
}

fn write_special_type(w: &mut impl Write, type_id: i8, data: i64) -> Result<(), ToMsgpackError> {
    // TODO: Elide leading zero bytes
    let data = data.to_be_bytes();
    rmp::encode::write_ext_meta(w, 8, type_id)?;
    w.write(&data)?;
    Ok(())
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
