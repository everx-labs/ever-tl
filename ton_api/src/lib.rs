#![allow(clippy::unreadable_literal)]
#![deny(private_in_public)]

use std::{fmt, io};
use std::any::Any;
use std::cmp::Ordering;
use std::convert::TryInto;
use std::fmt::{Display, Formatter};
use std::hash::Hash;

use erased_serde::serialize_trait_object;
use failure::Fail;
use serde_derive::{Deserialize, Serialize};

use ton_block::ShardIdent;
pub use ton_types::Result;
use ton_types::UInt256;

use crate::ton::ton_node::blockidext::BlockIdExt;

macro_rules! _invalid_id {
    ($id:ident) => {
        Err(crate::InvalidConstructor { expected: Self::possible_constructors(), received: $id }.into())
    };
}

#[allow(non_camel_case_types)]
pub mod ton;
pub mod secure;
mod ton_prelude;

/// Struct representing TL constructor number (CRC32 calculated from constructor definition string)
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ConstructorNumber(pub u32);

impl fmt::Debug for ConstructorNumber {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "#{:08x}", self.0)
    }
}

/// Struct for handling mismatched constructor number
#[derive(Debug, Fail)]
#[fail(display = "expected a constructor in {:?}; got {:?}", expected, received)]
pub struct InvalidConstructor {
    pub expected: Vec<ConstructorNumber>,
    pub received: ConstructorNumber,
}

/// Struct for deserializing TL-scheme objects from any `io::Read`
pub struct Deserializer<'r> {
    reader: &'r mut dyn io::Read,
}

impl<'r> Deserializer<'r> {
    /// Create `Deserializer` with given `io::Read` trait object
    pub fn new(reader: &'r mut dyn io::Read) -> Self {
        Deserializer { reader }
    }

    /// Read `ConstructorNumber` from reader
    pub fn read_constructor(&mut self) -> Result<ConstructorNumber> {
        use byteorder::{LittleEndian, ReadBytesExt};
        Ok(ConstructorNumber(self.read_u32::<LittleEndian>()?))
    }

    /// Read bare-serialized TL-object
    #[inline(always)]
    pub fn read_bare<D: BareDeserialize>(&mut self) -> Result<D> {
        D::deserialize_bare(self)
    }

    /// Read boxed-serialized TL-object
    #[inline(always)]
    pub fn read_boxed<D: BoxedDeserialize>(&mut self) -> Result<D> {
        let constructor = self.read_constructor()?;
        D::deserialize_boxed(constructor, self)
    }

    /// Returns default value for type
    #[inline(always)]
    pub fn just_default<D: Default>(&self) -> Result<D> {
        Ok(Default::default())
    }
}

impl<'r> io::Read for Deserializer<'r> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.reader.read(buf)
    }
}

/// Trait for bare type deserialization
pub trait BareDeserialize
where Self: Sized,
{
    /// Read bare-serialized value using `Deserializer`
    fn deserialize_bare(de: &mut Deserializer) -> Result<Self>;

    /// Read bare-serialized value from `u8` array
    fn bare_deserialized_from_bytes(mut bytes: &[u8]) -> Result<Self> {
        Deserializer::new(&mut bytes).read_bare()
    }
}

/// Trait for boxed type deserialization
pub trait BoxedDeserialize
where Self: Sized,
{
    /// Returns all possible constructors of boxed type
    fn possible_constructors() -> Vec<ConstructorNumber>;

    /// Read boxed-serialized value using `Deserializer`
    fn deserialize_boxed(id: ConstructorNumber, de: &mut Deserializer) -> Result<Self>;

    /// Read boxed-serialized value from `u8` array
    fn boxed_deserialized_from_bytes(mut bytes: &[u8]) -> Result<Self> {
        Deserializer::new(&mut bytes).read_boxed()
    }
}

/// Trait for deserializing any value represented `Object` TL type
pub trait BoxedDeserializeDynamic: BoxedDeserialize + for<'de> serde::Deserialize<'de> {
    /// Read boxed type value with given `ConstructorNumber` using `Deserializer`
    fn boxed_deserialize_to_box(id: ConstructorNumber, de: &mut Deserializer) -> Result<ton::TLObject>;

    /// Read boxed type value using `serde::Deserializer`
    fn serde_deserialize_to_box(de: &mut dyn erased_serde::Deserializer) -> ::std::result::Result<ton::TLObject, erased_serde::Error>;
}

impl<D> BoxedDeserializeDynamic for D
where D: BoxedDeserialize + for<'de> serde::Deserialize<'de> + AnyBoxedSerialize,
{
    fn boxed_deserialize_to_box(id: ConstructorNumber, de: &mut Deserializer) -> Result<ton::TLObject> {
        Ok(ton::TLObject::new(D::deserialize_boxed(id, de)?))
    }

    fn serde_deserialize_to_box(de: &mut dyn erased_serde::Deserializer) -> ::std::result::Result<ton::TLObject, erased_serde::Error>
    {
        Ok(ton::TLObject::new(erased_serde::deserialize::<D>(de)?))
    }
}

/// Struct representing every boxed type for deserializing `Object` TL type
#[derive(Clone, Copy)]
pub struct DynamicDeserializer {
    id: ConstructorNumber,
    type_name: &'static str,
    ton: fn(ConstructorNumber, &mut Deserializer) -> Result<ton::TLObject>,
}

impl DynamicDeserializer {
    #[inline(always)]
    pub fn from<D: BoxedDeserializeDynamic>(id: ConstructorNumber, type_name: &'static str) -> Self {
        DynamicDeserializer {
            id,
            type_name,
            ton: D::boxed_deserialize_to_box,
        }
    }
}

/// Struct for serializing TL-scheme objects into any `io::Write`
pub struct Serializer<'w> {
    writer: &'w mut dyn io::Write,
}

impl<'w> Serializer<'w> {
    /// Create `Serializer` with given `io::Write` trait object
    pub fn new(writer: &'w mut dyn io::Write) -> Self {
        Serializer { writer }
    }

    /// Read `ConstructorNumber` into writer
    pub fn write_constructor(&mut self, id: ConstructorNumber) -> Result<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        self.write_u32::<LittleEndian>(id.0)?;
        Ok(())
    }

    /// Serialize TL-object as bare value
    #[inline(always)]
    pub fn write_bare<S: ?Sized + BareSerialize>(&mut self, obj: &S) -> Result<()> {
        obj.serialize_bare(self)
    }

    /// Serialize TL-object as boxed value
    #[inline(always)]
    pub fn write_boxed<S: ?Sized + BoxedSerialize>(&mut self, obj: &S) -> Result<()> {
        let (constructor, bare) = obj.serialize_boxed();
        self.write_constructor(constructor)?;
        self.write_bare(bare)?;
        Ok(())
    }
}

impl<'w> io::Write for Serializer<'w> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.writer.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

/// Trait for bare type serialization
pub trait BareSerialize {
    /// Write object as bare-serialized value using `Serializer`
    fn serialize_bare(&self, ser: &mut Serializer) -> Result<()>;

    /// Write object as bare-serialized value into `Vec<u8>`
    fn bare_serialized_bytes(&self) -> Result<Vec<u8>> {
        let mut buf: Vec<u8> = vec![];
        Serializer::new(&mut buf).write_bare(self)?;
        Ok(buf)
    }
}

/// Trait for boxed type serialization
pub trait BoxedSerialize {
    /// Represent boxed type value as `ConstructorNumber` and `BareSerialize` tuple 
    fn serialize_boxed(&self) -> (ConstructorNumber, &dyn BareSerialize);

    /// Serialize boxed type value into `Vec<u8>`
    fn boxed_serialized_bytes(&self) -> Result<Vec<u8>> {
        let mut buf: Vec<u8> = vec![];
        Serializer::new(&mut buf).write_boxed(self)?;
        Ok(buf)
    }
}

/// Trait for representing bare types as boxed type
pub trait IntoBoxed: BareSerialize {
    type Boxed: BoxedSerialize;
    fn into_boxed(self) -> Self::Boxed;
}

/// Trait for representing any boxed type used in `Object` TL type processing
pub trait AnyBoxedSerialize: Any + Send + Sync + BoxedSerialize + erased_serde::Serialize {
    fn as_any(&self) -> &dyn Any;
    fn into_boxed_any(self: Box<Self>) -> Box<dyn Any + Send>;
}

impl<T: Any + Send + Sync + BoxedSerialize + erased_serde::Serialize> AnyBoxedSerialize for T {
    fn as_any(&self) -> &dyn Any { self }
    fn into_boxed_any(self: Box<Self>) -> Box<dyn Any + Send> { self }
}

serialize_trait_object!(AnyBoxedSerialize);

/// Trait for functional TL types
pub trait Function: AnyBoxedSerialize + serde::Serialize {
    type Reply: BoxedDeserialize + AnyBoxedSerialize;
}

impl PartialOrd for BlockIdExt {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BlockIdExt {
    fn cmp(&self, other: &Self) -> Ordering {
        self.workchain.cmp(&other.workchain)
            .then(self.shard.cmp(&other.shard))
            .then(self.seqno.cmp(&other.seqno))
            .then(self.root_hash.cmp(&other.root_hash))
            .then(self.file_hash.cmp(&other.file_hash))
    }
}

impl Display for BlockIdExt {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "({}:{:016x}, {}, rh {}, fh {})", self.workchain, self.shard, self.seqno, hex::encode(self.root_hash.0), hex::encode(self.file_hash.0))
    }
}

impl From<&ton_block::BlockIdExt> for BlockIdExt {
    fn from(block_id_ext: &ton_block::BlockIdExt) -> Self {
        Self {
            workchain: block_id_ext.shard().workchain_id(),
            shard: block_id_ext.shard().shard_prefix_with_tag() as i64,
            seqno: block_id_ext.seq_no() as i32,
            root_hash: block_id_ext.root_hash().into(),
            file_hash: block_id_ext.file_hash().into(),
        }
    }
}

impl From<ton_block::BlockIdExt> for BlockIdExt {
    fn from(block_id_ext: ton_block::BlockIdExt) -> Self {
        Self::from(&block_id_ext)
    }
}

impl TryInto<ton_block::BlockIdExt> for &BlockIdExt {
    type Error = failure::Error;

    fn try_into(self) -> Result<ton_block::BlockIdExt> {
        Ok(ton_block::BlockIdExt::with_params(
            ShardIdent::with_tagged_prefix(self.workchain, self.shard as u64)?,
            self.seqno as u32,
            self.root_hash.into(),
            self.file_hash.into(),
        ))
    }
}

impl Display for crate::ton::int256 {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl From<UInt256> for crate::ton::int256 {
    fn from(value: UInt256) -> Self {
        Self(value.into())
    }
}

impl From<&UInt256> for crate::ton::int256 {
    fn from(value: &UInt256) -> Self {
        Self(value.clone().into())
    }
}

impl Into<UInt256> for crate::ton::int256 {
    fn into(self) -> UInt256 {
        UInt256::from(self.0)
    }
}

impl Into<UInt256> for &crate::ton::int256 {
    fn into(self) -> UInt256 {
        UInt256::from(self.0)
    }
}
