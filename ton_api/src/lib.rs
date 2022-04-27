/*
* Copyright (C) 2019-2021 TON Labs. All Rights Reserved.
*
* Licensed under the SOFTWARE EVALUATION License (the "License"); you may not use
* this file except in compliance with the License.
*
* Unless required by applicable law or agreed to in writing, software
* distributed under the License is distributed on an "AS IS" BASIS,
* WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
* See the License for the specific TON DEV software governing permissions and
* limitations under the License.
*/

#![allow(clippy::unreadable_literal)]
#![deny(private_in_public)]

use crate::ton_prelude::TLObject;
use failure::Fail;
use std::{any::Any, fmt, hash::Hash, io::{self, Read, Write}};

use ton_block::{BlockIdExt, ShardIdent};
use ton_types::Result;
use ton_types::{fail, UInt256};

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
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
    reader: &'r mut dyn Read,
}

impl<'r> Deserializer<'r> {
    /// Create `Deserializer` with given `io::Read` trait object
    pub fn new(reader: &'r mut dyn Read) -> Self {
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

impl<'r> Read for Deserializer<'r> {
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
pub trait BoxedDeserializeDynamic: BoxedDeserialize {
    /// Read boxed type value with given `ConstructorNumber` using `Deserializer`
    fn boxed_deserialize_to_box(id: ConstructorNumber, de: &mut Deserializer) -> Result<ton::TLObject>;
}

impl<D> BoxedDeserializeDynamic for D
where D: BoxedDeserialize + AnyBoxedSerialize,
{
    fn boxed_deserialize_to_box(id: ConstructorNumber, de: &mut Deserializer) -> Result<ton::TLObject> {
        Ok(ton::TLObject::new(D::deserialize_boxed(id, de)?))
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
    writer: &'w mut dyn Write,
}

impl<'w> Serializer<'w> {
    /// Create `Serializer` with given `io::Write` trait object
    pub fn new(writer: &'w mut dyn Write) -> Self {
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

    #[inline(always)]
    pub fn write_into_boxed<S: BareSerialize>(&mut self, obj: &S) -> Result<()> {
        let constructor = obj.constructor();
        self.write_constructor(constructor)?;
        self.write_bare(obj)?;
        Ok(())
    }
}

impl<'w> Write for Serializer<'w> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.writer.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

/// Trait for bare type serialization
pub trait BareSerialize {
    /// Get constructor id for object (tl_id)
    fn constructor(&self) -> crate::ConstructorNumber;

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
pub trait AnyBoxedSerialize: Any + Send + Sync + BoxedSerialize {
    fn as_any(&self) -> &dyn Any;
    fn into_boxed_any(self: Box<Self>) -> Box<dyn Any + Send>;
}

impl<T: Any + Send + Sync + BoxedSerialize> AnyBoxedSerialize for T {
    fn as_any(&self) -> &dyn Any { self }
    fn into_boxed_any(self: Box<Self>) -> Box<dyn Any + Send> { self }
}

/// Trait for functional TL types
pub trait Function: AnyBoxedSerialize {
    type Reply: BoxedDeserialize + AnyBoxedSerialize;
}

impl BareDeserialize for BlockIdExt {
    fn deserialize_bare(de: &mut Deserializer) -> Result<Self> {
        let shard = ShardIdent::with_tagged_prefix(
            de.read_bare::<crate::ton::int>()?,
            de.read_bare::<crate::ton::long>()? as u64
        )?;
        let ret = Self::with_params(
            shard, 
            de.read_bare::<crate::ton::int>()? as u32, 
            de.read_bare::<UInt256>()?,
            de.read_bare::<UInt256>()? 
        );
        Ok(ret)
    }
}

impl BareSerialize for BlockIdExt {
    fn constructor(&self) -> ConstructorNumber {
        crate::ton::ton_node::blockidext::TL_TAG
    }
    fn serialize_bare(&self, se: &mut Serializer) -> Result<()> {
        let shard = self.shard();
        se.write_bare::<crate::ton::int>(&shard.workchain_id())?;
        se.write_bare::<crate::ton::long>(&(shard.shard_prefix_with_tag() as i64))?;
        se.write_bare::<crate::ton::int>(&(self.seq_no() as i32))?;
        se.write_bare::<UInt256>(self.root_hash())?;
        se.write_bare::<UInt256>(self.file_hash())?;
        Ok(())
    }
}

impl BoxedDeserialize for BlockIdExt {
    fn possible_constructors() -> Vec<crate::ConstructorNumber> {
        vec![crate::ton::ton_node::blockidext::TL_TAG]
    }
    fn deserialize_boxed(id: ConstructorNumber, de: &mut Deserializer) -> Result<Self> {
        if id == crate::ton::ton_node::blockidext::TL_TAG {
            de.read_bare()
        } else {
            _invalid_id!(id)
        }
    }
}

impl BoxedSerialize for BlockIdExt {
    fn serialize_boxed(&self) -> (ConstructorNumber, &dyn BareSerialize) {
        (crate::ton::ton_node::blockidext::TL_TAG, self)
    }
}

impl BareDeserialize for UInt256 {
    fn deserialize_bare(de: &mut Deserializer) -> Result<Self> {
        let mut data = [0u8; 32];
        de.read_exact(&mut data)?;
        Ok(Self::with_array(data))
    }
}

impl BareSerialize for UInt256 {
    fn constructor(&self) -> ConstructorNumber { 
        unreachable!() 
    }
    fn serialize_bare(&self, se: &mut Serializer) -> Result<()> {
        se.write_all(self.as_slice())?;
        Ok(())
    }
}

// Deserialize boxed TL object from bytes
pub fn deserialize_boxed(bytes: &[u8]) -> Result<TLObject> {
    let mut reader = bytes;
    Deserializer::new(&mut reader).read_boxed::<TLObject>()
}

/// Deserialize bundle of boxed TL objects from bytes
pub fn deserialize_boxed_bundle(bytes: &[u8]) -> Result<Vec<TLObject>> {
    let mut reader = bytes;
    let mut de = Deserializer::new(&mut reader);
    let mut ret = Vec::new();
    loop {
        match de.read_boxed::<TLObject>() {
            Ok(object) => ret.push(object),
            Err(_) => if ret.is_empty() {
                fail!("Deserialization error")
            } else {
                break
            }
        }
    }
    Ok(ret)
}

/// Serialize boxed TL object into bytes
pub fn serialize_boxed<T: BoxedSerialize>(object: &T) -> Result<Vec<u8>> {
    let mut ret = Vec::new();
    Serializer::new(&mut ret).write_boxed(object)?;
    Ok(ret)
}

/// Serialize boxed TL object into bytes with appending
pub fn serialize_boxed_append<T: BoxedSerialize>(buf: &mut Vec<u8>, object: &T) -> Result<()> {
    Serializer::new(buf).write_boxed(object)?;
    Ok(())
}

/// Serialize boxed TL object into bytes in-place
pub fn serialize_boxed_inplace<T: BoxedSerialize>(buf: &mut Vec<u8>, object: &T) -> Result<()> {
    buf.truncate(0); 
    serialize_boxed_append(buf, object)
}

/// Serialize non-boxed TL object into bytes
pub fn serialize_bare<T: BareSerialize>(object: &T) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    Serializer::new(&mut buf).write_into_boxed(object)?;
    Ok(buf)
}

/// Serialize non-boxed TL object into bytes in-place
pub fn serialize_bare_inplace<T: BareSerialize>(buf: &mut Vec<u8>, object: &T) -> Result<()> {
    buf.truncate(0);
    Serializer::new(buf).write_into_boxed(object)
}

/// Get TL tag from non-boxed object
pub fn tag_from_bare_object<T: BareSerialize>(object: &T) -> u32 {
    let ConstructorNumber(tag) = object.constructor();
    tag
}

/// Get TL tag from non-boxed type
pub fn tag_from_bare_type<T: Default + IntoBoxed>() -> u32 {
    let (ConstructorNumber(tag), _) = T::default().into_boxed().serialize_boxed();
    tag
}

/// Get TL tag from boxed object
pub fn tag_from_boxed_object<T: BoxedSerialize>(object: &T) -> u32 {
    let (ConstructorNumber(tag), _) = object.serialize_boxed();
    tag
}

/// Get TL tag from boxed type
pub fn tag_from_boxed_type<T: Default + BoxedSerialize>() -> u32 {
    let (ConstructorNumber(tag), _) = T::default().serialize_boxed();
    tag
}

/// Get TL tag from data bytes
pub fn tag_from_data(data: &[u8]) -> u32 {
    if data.len() < 4 {
        0
    } else {
        u32::from_le_bytes([data[0], data[1], data[2], data[3]])
    }
}