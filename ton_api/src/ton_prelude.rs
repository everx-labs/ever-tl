#![allow(non_camel_case_types)]

use std::fmt;
use std::io::{Read, Write};
use std::marker::PhantomData;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use extfmt::Hexlify;
use ordered_float::OrderedFloat;
use rand::{Rand, Rng};
use serde;
use serde_derive::{Deserialize, Serialize};

use crate::{AnyBoxedSerialize, BareDeserialize, BareSerialize, BoxedDeserialize, BoxedSerialize, ConstructorNumber, Deserializer, Result, Serializer};
use crate::ton::Bool;

const MAX_BYTES_DEBUG_LEN: usize = 4;

macro_rules! impl_byteslike {
    (@common $ty:ident) => {

        impl fmt::Debug for $ty {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                if self.len() <= MAX_BYTES_DEBUG_LEN {
                    write!(f, "<{}>", Hexlify(&self.0))
                } else {
                    write!(f, "<{}... {} bytes>", Hexlify(&self.0[..MAX_BYTES_DEBUG_LEN]), self.0.len())
                }
            }
        }

        impl ::std::ops::Deref for $ty {
            type Target = [u8];
            fn deref(&self) -> &[u8] { &self.0 }
        }

        impl ::std::ops::DerefMut for $ty {
            fn deref_mut(&mut self) -> &mut [u8] { &mut self.0 }
        }

    };

    (@arraylike $ty:ident) => {

        impl_byteslike!(@common $ty);

        impl Rand for $ty {
            fn rand<R: Rng>(rng: &mut R) -> Self {
                let mut ret: Self = Default::default();
                rng.fill_bytes(&mut ret.0);
                ret
            }
        }

        impl BareDeserialize for $ty {
            fn deserialize_bare(de: &mut Deserializer) -> Result<Self> {
                let mut ret: Self = Default::default();
                de.read_exact(&mut ret.0)?;
                Ok(ret)
            }
        }

        impl BareSerialize for $ty {
            fn serialize_bare(&self, ser: &mut Serializer) -> Result<()> {
                ser.write_all(&self.0)?;
                Ok(())
            }
        }

    };
}

/// Represents bytes vector.
#[derive(Clone, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
pub struct bytes(pub Vec<u8>);

impl BareDeserialize for bytes {
    fn deserialize_bare(de: &mut Deserializer) -> Result<Self> {
        let vec = de.read_bare::<Vec<u8>>()?;
        Ok(bytes(vec))
    }
}

impl BareSerialize for bytes {
    fn serialize_bare(&self, ser: &mut Serializer) -> Result<()> {
        ser.write_bare::<[u8]>(&self.0)
    }
}

impl From<Vec<u8>> for bytes {
    fn from(v: Vec<u8>) -> Self {
        bytes(v)
    }
}

/// Represents 128-bit unsigned integer.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
pub struct int128(pub [u8; 16]);

/// Represents 256-bit unsigned integer.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
pub struct int256(pub [u8; 32]);

impl_byteslike!(@common bytes);
impl_byteslike!(@arraylike int128);
impl_byteslike!(@arraylike int256);

/// Represents base TL-object type.
pub struct TLObject(Box<dyn AnyBoxedSerialize>);

impl TLObject {
    pub fn new<I: AnyBoxedSerialize>(inner: I) -> Self {
        TLObject(Box::new(inner))
    }

    pub fn is<I: AnyBoxedSerialize>(&self) -> bool {
        self.0.as_any().is::<I>()
    }

    pub fn downcast<I: AnyBoxedSerialize>(self) -> ::std::result::Result<I, Self> {
        if self.is::<I>() {
            Ok(*self.0.into_boxed_any().downcast::<I>().unwrap())
        } else {
            Err(self)
        }
    }
}

impl Clone for TLObject {
    fn clone(&self) -> Self {
        unimplemented!()
    }
}

impl fmt::Debug for TLObject {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (type_id, _) = self.0.serialize_boxed();
        write!(f, "(TLObject tl_id:{:?})", type_id)
    }
}

impl BoxedDeserialize for TLObject {
    fn possible_constructors() -> Vec<ConstructorNumber> {
        crate::ton::dynamic::BY_NUMBER.keys().cloned().collect()
    }

    fn deserialize_boxed(id: ConstructorNumber, de: &mut Deserializer) -> Result<Self> {
        match crate::ton::dynamic::BY_NUMBER.get(&id) {
            Some(dynamic) => (dynamic.ton)(id, de),
            None => _invalid_id!(id),
        }
    }
}

impl BoxedSerialize for TLObject {
    fn serialize_boxed(&self) -> (ConstructorNumber, &dyn BareSerialize) {
        self.0.serialize_boxed()
    }
}

impl serde::Serialize for TLObject {
    fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
        where S: serde::Serializer,
    {
        let (id, _) = self.0.serialize_boxed();
        let tl_type_name = crate::ton::dynamic::BY_NUMBER.get(&id)
            .map(|dynamic| dynamic.type_name)
            .unwrap_or(&"<bogus>");
        serializer.serialize_newtype_variant("TLObject", id.0, tl_type_name, &self.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Hash)]
pub struct LengthPrefixed<T>(pub T);

impl<T> From<T> for LengthPrefixed<T> {
    fn from(x: T) -> Self {
        LengthPrefixed(x)
    }
}

impl<T> BareDeserialize for LengthPrefixed<T>
    where T: BoxedDeserialize,
{
    fn deserialize_bare(de: &mut Deserializer) -> Result<Self> {
        let len = de.read_i32::<LittleEndian>()? as usize;
        let mut buf = vec![0u8; len];
        de.read_exact(&mut buf)?;
        Ok(LengthPrefixed(T::boxed_deserialized_from_bytes(&buf)?))
    }
}

impl<T> BareSerialize for LengthPrefixed<T>
    where T: BoxedSerialize,
{
    fn serialize_bare(&self, ser: &mut Serializer) -> Result<()> {
        let inner = self.0.boxed_serialized_bytes()?;
        ser.write_i32::<LittleEndian>(inner.len() as i32)?;
        ser.write_all(&inner)?;
        Ok(())
    }
}

impl BareSerialize for () {
    fn serialize_bare(&self, _ser: &mut Serializer) -> Result<()> {
        Ok(())
    }
}

impl From<bool> for &'static Bool {
    fn from(b: bool) -> Self {
        if b { &Bool::BoolTrue } else { &Bool::BoolFalse }
    }
}

impl From<bool> for Bool {
    fn from(b: bool) -> Self {
        let b: &'static Bool = b.into();
        b.clone()
    }
}

impl Into<bool> for Bool {
    fn into(self) -> bool {
        match self {
            Bool::BoolTrue => true,
            Bool::BoolFalse => false,
        }
    }
}

impl BoxedDeserialize for bool {
    fn possible_constructors() -> Vec<ConstructorNumber> {
        Bool::possible_constructors()
    }

    fn deserialize_boxed(id: ConstructorNumber, de: &mut Deserializer) -> Result<Self> {
        Ok(Bool::deserialize_boxed(id, de)?.into())
    }
}

impl BoxedSerialize for bool {
    fn serialize_boxed(&self) -> (ConstructorNumber, &dyn BareSerialize) {
        let b: &'static Bool = (*self).into();
        Bool::serialize_boxed(b)
    }
}

impl BareDeserialize for String {
    fn deserialize_bare(de: &mut Deserializer) -> Result<Self> {
        let vec = de.read_bare::<Vec<u8>>()?;
        Ok(String::from_utf8(vec)?)
    }
}

impl BareSerialize for String {
    fn serialize_bare(&self, ser: &mut Serializer) -> Result<()> {
        ser.write_bare::<[u8]>(self.as_bytes())?;
        Ok(())
    }
}

impl<T> BoxedDeserialize for Box<T>
    where T: BoxedDeserialize,
{
    fn possible_constructors() -> Vec<ConstructorNumber> {
        T::possible_constructors()
    }

    fn deserialize_boxed(id: ConstructorNumber, de: &mut Deserializer) -> Result<Self> {
        Ok(Box::new(T::deserialize_boxed(id, de)?))
    }
}

impl<T> BoxedSerialize for Box<T>
    where T: BoxedSerialize,
{
    fn serialize_boxed(&self) -> (ConstructorNumber, &dyn BareSerialize) {
        T::serialize_boxed(self)
    }
}

/// Base enumeration for any bare type. Used as vectors type parameter.
#[derive(PartialEq, Hash)]
pub enum Bare {
    None
}

impl Default for Bare {
    fn default() -> Self {
        Bare::None
    }
}

/// Base enumeration for any boxed type. Used as vectors type parameter.
#[derive(PartialEq, Hash)]
pub enum Boxed {
    None
}

impl Default for Boxed {
    fn default() -> Self {
        Boxed::None
    }
}

#[derive(PartialEq, Hash, Default)]
pub struct Vector<Det, T>(pub Vec<T>, PhantomData<fn() -> Det>);
pub type vector<Det, T> = Vector<Det, T>;

impl<Det, T> Clone for Vector<Det, T>
    where T: Clone,
{
    fn clone(&self) -> Self {
        Vector(self.0.clone(), PhantomData)
    }
}

impl<Det, T> fmt::Debug for Vector<Det, T>
    where T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("Vector")
            .field(&self.0)
            .finish()
    }
}

impl<Det, T> serde::Serialize for Vector<Det, T>
    where T: serde::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
        where S: serde::Serializer,
    {
        use serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
        for item in &self.0 {
            seq.serialize_element(item)?;
        }
        seq.end()
    }
}

impl<'de, Det, T> serde::Deserialize<'de> for Vector<Det, T>
    where T: serde::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where D: serde::Deserializer<'de>,
    {
        Ok(Vector(serde::Deserialize::deserialize(deserializer)?, PhantomData))
    }
}

const VECTOR_CONSTRUCTOR: ConstructorNumber = ConstructorNumber(0x1cb5c415);

macro_rules! impl_vector {
    ($det:ident, $det_de:ident, $det_ser:ident, $read_method:ident, $write_method:ident) => {

        impl<T> From<Vec<T>> for Vector<$det, T> {
            fn from(obj: Vec<T>) -> Self {
                Vector(obj, PhantomData)
            }
        }

        impl<T> ::std::ops::Deref for Vector<$det, T> {
            type Target = [T];
            fn deref(&self) -> &[T] { &self.0 }
        }

        impl<T> ::std::ops::DerefMut for Vector<$det, T> {
            fn deref_mut(&mut self) -> &mut [T] { &mut self.0 }
        }

        impl<T> BareDeserialize for Vector<$det, T>
            where T: $det_de,
        {
            fn deserialize_bare(de: &mut Deserializer) -> Result<Self> {
                let count = de.read_i32::<LittleEndian>()?;
                let mut ret = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    ret.push(de.$read_method()?);
                }
                Ok(ret.into())
            }
        }

        impl<T> BoxedDeserialize for Vector<$det, T>
            where Self: BareDeserialize,
        {
            fn possible_constructors() -> Vec<ConstructorNumber> { vec![VECTOR_CONSTRUCTOR] }

            fn deserialize_boxed(id: ConstructorNumber, de: &mut Deserializer) -> Result<Self> {
                assert_eq!(id, VECTOR_CONSTRUCTOR);
                Self::deserialize_bare(de)
            }
        }

        impl<T> BareSerialize for Vector<$det, T>
            where T: $det_ser,
        {
            fn serialize_bare(&self, ser: &mut Serializer) -> Result<()> {
                ser.write_i32::<LittleEndian>(self.0.len() as i32)?;
                for item in &self.0 {
                    ser.$write_method(item)?;
                }
                Ok(())
            }
        }

        impl<T> BoxedSerialize for Vector<$det, T>
            where Self: BareSerialize,
        {
            fn serialize_boxed(&self) -> (ConstructorNumber, &dyn BareSerialize) {
                (VECTOR_CONSTRUCTOR, self)
            }
        }

    }
}

impl_vector! { Bare, BareDeserialize, BareSerialize, read_bare, write_bare }
impl_vector! { Boxed, BoxedDeserialize, BoxedSerialize, read_boxed, write_boxed }

impl BareDeserialize for Vec<u8> {
    fn deserialize_bare(de: &mut Deserializer) -> Result<Self> {
        let len = de.read_u8()?;
        let (len, mut have_read) = if len != 254 {
            (len as usize, 1)
        } else {
            (de.read_u24::<LittleEndian>()? as usize, 4)
        };

        let mut buf = vec![0; len];
        de.read_exact(&mut buf)?;
        have_read += len;
        let remainder = have_read % 4;
        if remainder != 0 {
            let mut buf = [0u8; 4];
            de.read_exact(&mut buf[remainder..])?;
        }
        Ok(buf)
    }
}

impl BareSerialize for [u8] {
    fn serialize_bare(&self, ser: &mut Serializer) -> Result<()> {
        let len = self.len();
        let mut have_written = if len < 254 {
            ser.write_u8(len as u8)?;
            1
        } else {
            ser.write_u8(254)?;
            ser.write_u24::<LittleEndian>(len as u32)?;
            4
        };

        ser.write_all(self)?;
        have_written += len;
        let remainder = have_written % 4;
        if remainder != 0 {
            let buf = [0u8; 4];
            ser.write_all(&buf[remainder..])?;
        }
        Ok(())
    }
}

macro_rules! impl_tl_primitive {
    ($tltype:ident, $ptype:ty, $read:ident, $write:ident) => {
        pub type $tltype = $ptype;

        impl BareDeserialize for $ptype {
            fn deserialize_bare(de: &mut Deserializer) -> Result<Self> {
                Ok(de.$read::<LittleEndian>()?)
            }
        }

        impl BareSerialize for $ptype {
            fn serialize_bare(&self, ser: &mut Serializer) -> Result<()> {
                ser.$write::<LittleEndian>(*self)?;
                Ok(())
            }
        }
    }
}

impl_tl_primitive! { int, i32, read_i32, write_i32 }
impl_tl_primitive! { uint, u32, read_u32, write_u32 }
impl_tl_primitive! { long, i64, read_i64, write_i64 }
impl_tl_primitive! { ulong, u64, read_u64, write_u64 }

pub type double = OrderedFloat<f64>;

impl BareDeserialize for double {
    fn deserialize_bare(de: &mut Deserializer) -> Result<Self> {
        Ok(de.read_f64::<LittleEndian>()?.into())
    }
}

impl BareSerialize for double {
    fn serialize_bare(&self, ser: &mut Serializer) -> Result<()> {
        ser.write_f64::<LittleEndian>(self.0)?;
        Ok(())
    }
}

// Built-in types:
pub type Int32 = i32;
pub type Int53 = i64;
pub type Int64 = i64;

pub type int32 = Int32;
pub type int53 = Int53;
pub type int64 = Int64;

/// Flags built-in type.
pub type Flags = u32;
pub type lengthPrefixedTypedObject = LengthPrefixed<TypedObject>;
pub type True = bool;
/// String built-in type.
pub type string = String;
/// Alias of TLObject built-in type.
pub type TypedObject = TLObject;
/// Alias of TLObject built-in type.
pub type Object = TLObject;
/// Function. Alias of TLObject built-in type.
pub type Function = TLObject;
/// Alias of SecureBytes built-in type.
pub type secureBytes = crate::secure::SecureBytes;
/// Alias of SecureString built-in type.
pub type secureString = crate::secure::SecureString;
