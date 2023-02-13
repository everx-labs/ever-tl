use serde_derive::{Deserialize, Serialize};
#[derive(Debug, Default, Clone, PartialEq)]
#[doc = "TL-derived from `smc.getCode`\n\n```text\nsmc.getCode id:int53 = tvm.Cell;\n```\n"]
pub struct GetCode {
    pub id: crate::ton::int53,
}
impl Eq for GetCode {}
impl crate::BareSerialize for GetCode {
    fn constructor(&self) -> crate::ConstructorNumber {
        crate::ConstructorNumber(0x81e61b98)
    }
    fn serialize_bare(&self, _ser: &mut crate::Serializer) -> crate::Result<()> {
        let &GetCode { ref id } = self;
        _ser.write_bare::<crate::ton::int53>(id)?;
        Ok(())
    }
}
impl crate::BareDeserialize for GetCode {
    fn deserialize_bare(_de: &mut crate::Deserializer) -> crate::Result<Self> {
        {
            let id = _de.read_bare::<crate::ton::int53>()?;
            Ok(Self { id })
        }
    }
}
impl crate::BoxedDeserialize for GetCode {
    fn possible_constructors() -> Vec<crate::ConstructorNumber> {
        vec![crate::ConstructorNumber(0x81e61b98)]
    }
    fn deserialize_boxed(
        id: crate::ConstructorNumber,
        de: &mut crate::Deserializer,
    ) -> crate::Result<Self> {
        if id == crate::ConstructorNumber(0x81e61b98) {
            de.read_bare()
        } else {
            _invalid_id!(id)
        }
    }
}
impl crate::BoxedSerialize for GetCode {
    fn serialize_boxed(&self) -> (crate::ConstructorNumber, &dyn crate::BareSerialize) {
        (crate::ConstructorNumber(0x81e61b98), self)
    }
}
impl crate::Function for GetCode {
    type Reply = crate::ton::tvm::Cell;
}
#[derive(Debug, Default, Clone, PartialEq)]
#[doc = "TL-derived from `smc.getData`\n\n```text\nsmc.getData id:int53 = tvm.Cell;\n```\n"]
pub struct GetData {
    pub id: crate::ton::int53,
}
impl Eq for GetData {}
impl crate::BareSerialize for GetData {
    fn constructor(&self) -> crate::ConstructorNumber {
        crate::ConstructorNumber(0xe6835349)
    }
    fn serialize_bare(&self, _ser: &mut crate::Serializer) -> crate::Result<()> {
        let &GetData { ref id } = self;
        _ser.write_bare::<crate::ton::int53>(id)?;
        Ok(())
    }
}
impl crate::BareDeserialize for GetData {
    fn deserialize_bare(_de: &mut crate::Deserializer) -> crate::Result<Self> {
        {
            let id = _de.read_bare::<crate::ton::int53>()?;
            Ok(Self { id })
        }
    }
}
impl crate::BoxedDeserialize for GetData {
    fn possible_constructors() -> Vec<crate::ConstructorNumber> {
        vec![crate::ConstructorNumber(0xe6835349)]
    }
    fn deserialize_boxed(
        id: crate::ConstructorNumber,
        de: &mut crate::Deserializer,
    ) -> crate::Result<Self> {
        if id == crate::ConstructorNumber(0xe6835349) {
            de.read_bare()
        } else {
            _invalid_id!(id)
        }
    }
}
impl crate::BoxedSerialize for GetData {
    fn serialize_boxed(&self) -> (crate::ConstructorNumber, &dyn crate::BareSerialize) {
        (crate::ConstructorNumber(0xe6835349), self)
    }
}
impl crate::Function for GetData {
    type Reply = crate::ton::tvm::Cell;
}
#[derive(Debug, Default, Clone, PartialEq)]
#[doc = "TL-derived from `smc.getState`\n\n```text\nsmc.getState id:int53 = tvm.Cell;\n```\n"]
pub struct GetState {
    pub id: crate::ton::int53,
}
impl Eq for GetState {}
impl crate::BareSerialize for GetState {
    fn constructor(&self) -> crate::ConstructorNumber {
        crate::ConstructorNumber(0xf338a9eb)
    }
    fn serialize_bare(&self, _ser: &mut crate::Serializer) -> crate::Result<()> {
        let &GetState { ref id } = self;
        _ser.write_bare::<crate::ton::int53>(id)?;
        Ok(())
    }
}
impl crate::BareDeserialize for GetState {
    fn deserialize_bare(_de: &mut crate::Deserializer) -> crate::Result<Self> {
        {
            let id = _de.read_bare::<crate::ton::int53>()?;
            Ok(Self { id })
        }
    }
}
impl crate::BoxedDeserialize for GetState {
    fn possible_constructors() -> Vec<crate::ConstructorNumber> {
        vec![crate::ConstructorNumber(0xf338a9eb)]
    }
    fn deserialize_boxed(
        id: crate::ConstructorNumber,
        de: &mut crate::Deserializer,
    ) -> crate::Result<Self> {
        if id == crate::ConstructorNumber(0xf338a9eb) {
            de.read_bare()
        } else {
            _invalid_id!(id)
        }
    }
}
impl crate::BoxedSerialize for GetState {
    fn serialize_boxed(&self) -> (crate::ConstructorNumber, &dyn crate::BareSerialize) {
        (crate::ConstructorNumber(0xf338a9eb), self)
    }
}
impl crate::Function for GetState {
    type Reply = crate::ton::tvm::Cell;
}
#[derive(Debug, Default, Clone, PartialEq)]
#[doc = "TL-derived from `smc.load`\n\n```text\nsmc.load account_address:accountAddress = smc.Info;\n```\n"]
pub struct Load {
    pub account_address: crate::ton::accountaddress::AccountAddress,
}
impl Eq for Load {}
impl crate::BareSerialize for Load {
    fn constructor(&self) -> crate::ConstructorNumber {
        crate::ConstructorNumber(0xca25d03f)
    }
    fn serialize_bare(&self, _ser: &mut crate::Serializer) -> crate::Result<()> {
        let &Load {
            ref account_address,
        } = self;
        _ser.write_bare::<crate::ton::accountaddress::AccountAddress>(account_address)?;
        Ok(())
    }
}
impl crate::BareDeserialize for Load {
    fn deserialize_bare(_de: &mut crate::Deserializer) -> crate::Result<Self> {
        {
            let account_address = _de.read_bare::<crate::ton::accountaddress::AccountAddress>()?;
            Ok(Self { account_address })
        }
    }
}
impl crate::BoxedDeserialize for Load {
    fn possible_constructors() -> Vec<crate::ConstructorNumber> {
        vec![crate::ConstructorNumber(0xca25d03f)]
    }
    fn deserialize_boxed(
        id: crate::ConstructorNumber,
        de: &mut crate::Deserializer,
    ) -> crate::Result<Self> {
        if id == crate::ConstructorNumber(0xca25d03f) {
            de.read_bare()
        } else {
            _invalid_id!(id)
        }
    }
}
impl crate::BoxedSerialize for Load {
    fn serialize_boxed(&self) -> (crate::ConstructorNumber, &dyn crate::BareSerialize) {
        (crate::ConstructorNumber(0xca25d03f), self)
    }
}
impl crate::Function for Load {
    type Reply = crate::ton::smc::Info;
}
#[derive(Debug, Default, Clone, PartialEq)]
#[doc = "TL-derived from `smc.runGetMethod`\n\n```text\nsmc.runGetMethod id:int53 method:smc.MethodId stack:vector<tvm.StackEntry> = smc.RunResult;\n```\n"]
pub struct RunGetMethod {
    pub id: crate::ton::int53,
    pub method: crate::ton::smc::MethodId,
    pub stack: crate::ton::vector<crate::ton::Boxed, crate::ton::tvm::StackEntry>,
}
impl Eq for RunGetMethod {}
impl crate::BareSerialize for RunGetMethod {
    fn constructor(&self) -> crate::ConstructorNumber {
        crate::ConstructorNumber(0xf0c905aa)
    }
    fn serialize_bare(&self, _ser: &mut crate::Serializer) -> crate::Result<()> {
        let &RunGetMethod {
            ref id,
            ref method,
            ref stack,
        } = self;
        _ser.write_bare::<crate::ton::int53>(id)?;
        _ser.write_boxed::<crate::ton::smc::MethodId>(method)?;
        _ser.write_bare::<crate::ton::vector<crate::ton::Boxed, crate::ton::tvm::StackEntry>>(
            stack,
        )?;
        Ok(())
    }
}
impl crate::BareDeserialize for RunGetMethod {
    fn deserialize_bare(_de: &mut crate::Deserializer) -> crate::Result<Self> {
        {
            let id = _de.read_bare::<crate::ton::int53>()?;
            let method = _de.read_boxed::<crate::ton::smc::MethodId>()?;
            let stack = _de
                .read_bare::<crate::ton::vector<crate::ton::Boxed, crate::ton::tvm::StackEntry>>(
                )?;
            Ok(Self { id, method, stack })
        }
    }
}
impl crate::BoxedDeserialize for RunGetMethod {
    fn possible_constructors() -> Vec<crate::ConstructorNumber> {
        vec![crate::ConstructorNumber(0xf0c905aa)]
    }
    fn deserialize_boxed(
        id: crate::ConstructorNumber,
        de: &mut crate::Deserializer,
    ) -> crate::Result<Self> {
        if id == crate::ConstructorNumber(0xf0c905aa) {
            de.read_bare()
        } else {
            _invalid_id!(id)
        }
    }
}
impl crate::BoxedSerialize for RunGetMethod {
    fn serialize_boxed(&self) -> (crate::ConstructorNumber, &dyn crate::BareSerialize) {
        (crate::ConstructorNumber(0xf0c905aa), self)
    }
}
impl crate::Function for RunGetMethod {
    type Reply = crate::ton::smc::RunResult;
}
