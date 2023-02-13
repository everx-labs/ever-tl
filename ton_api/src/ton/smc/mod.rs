use serde_derive::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq)]
#[doc = "TL-derived from `smc.Info`\n\n```text\nsmc.info id:int53 = smc.Info;\n```\n"]
pub enum Info {
    Smc_Info(crate::ton::smc::info::Info),
}
impl Info {
    pub fn id(&self) -> &crate::ton::int53 {
        match self {
            &Info::Smc_Info(ref x) => &x.id,
        }
    }
    pub fn only(self) -> crate::ton::smc::info::Info {
        match self {
            Info::Smc_Info(x) => x,
        }
    }
}
impl Eq for Info {}
impl Default for Info {
    fn default() -> Self {
        Info::Smc_Info(crate::ton::smc::info::Info::default())
    }
}
impl crate::BoxedSerialize for Info {
    fn serialize_boxed(&self) -> (crate::ConstructorNumber, &dyn crate::BareSerialize) {
        match self {
            &Info::Smc_Info(ref x) => (crate::ConstructorNumber(0x439b963c), x),
        }
    }
}
impl crate::BoxedDeserialize for Info {
    fn possible_constructors() -> Vec<crate::ConstructorNumber> {
        vec![crate::ConstructorNumber(0x439b963c)]
    }
    fn deserialize_boxed(
        _id: crate::ConstructorNumber,
        _de: &mut crate::Deserializer,
    ) -> crate::Result<Self> {
        match _id {
            crate::ConstructorNumber(0x439b963c) => Ok(Info::Smc_Info(
                _de.read_bare::<crate::ton::smc::info::Info>()?,
            )),
            id => _invalid_id!(id),
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
#[doc = "TL-derived from `smc.MethodId`\n\n```text\nsmc.methodIdName name:string = smc.MethodId;\n\nsmc.methodIdNumber number:int32 = smc.MethodId;\n```\n"]
pub enum MethodId {
    Smc_MethodIdName(crate::ton::smc::methodid::MethodIdName),
    Smc_MethodIdNumber(crate::ton::smc::methodid::MethodIdNumber),
}
impl MethodId {
    pub fn name(&self) -> Option<&crate::ton::string> {
        match self {
            &MethodId::Smc_MethodIdName(ref x) => Some(&x.name),
            _ => None,
        }
    }
    pub fn number(&self) -> Option<&crate::ton::int32> {
        match self {
            &MethodId::Smc_MethodIdNumber(ref x) => Some(&x.number),
            _ => None,
        }
    }
}
impl Eq for MethodId {}
impl Default for MethodId {
    fn default() -> Self {
        MethodId::Smc_MethodIdName(crate::ton::smc::methodid::MethodIdName::default())
    }
}
impl crate::BoxedSerialize for MethodId {
    fn serialize_boxed(&self) -> (crate::ConstructorNumber, &dyn crate::BareSerialize) {
        match self {
            &MethodId::Smc_MethodIdName(ref x) => (crate::ConstructorNumber(0xf127ff94), x),
            &MethodId::Smc_MethodIdNumber(ref x) => (crate::ConstructorNumber(0xa423b9fc), x),
        }
    }
}
impl crate::BoxedDeserialize for MethodId {
    fn possible_constructors() -> Vec<crate::ConstructorNumber> {
        vec![
            crate::ConstructorNumber(0xf127ff94),
            crate::ConstructorNumber(0xa423b9fc),
        ]
    }
    fn deserialize_boxed(
        _id: crate::ConstructorNumber,
        _de: &mut crate::Deserializer,
    ) -> crate::Result<Self> {
        match _id {
            crate::ConstructorNumber(0xf127ff94) => Ok(MethodId::Smc_MethodIdName(
                _de.read_bare::<crate::ton::smc::methodid::MethodIdName>()?,
            )),
            crate::ConstructorNumber(0xa423b9fc) => Ok(MethodId::Smc_MethodIdNumber(
                _de.read_bare::<crate::ton::smc::methodid::MethodIdNumber>()?,
            )),
            id => _invalid_id!(id),
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
#[doc = "TL-derived from `smc.RunResult`\n\n```text\nsmc.runResult gas_used:int53 stack:vector<tvm.StackEntry> exit_code:int32 = smc.RunResult;\n```\n"]
pub enum RunResult {
    Smc_RunResult(crate::ton::smc::runresult::RunResult),
}
impl RunResult {
    pub fn exit_code(&self) -> &crate::ton::int32 {
        match self {
            &RunResult::Smc_RunResult(ref x) => &x.exit_code,
        }
    }
    pub fn gas_used(&self) -> &crate::ton::int53 {
        match self {
            &RunResult::Smc_RunResult(ref x) => &x.gas_used,
        }
    }
    pub fn stack(&self) -> &crate::ton::vector<crate::ton::Boxed, crate::ton::tvm::StackEntry> {
        match self {
            &RunResult::Smc_RunResult(ref x) => &x.stack,
        }
    }
    pub fn only(self) -> crate::ton::smc::runresult::RunResult {
        match self {
            RunResult::Smc_RunResult(x) => x,
        }
    }
}
impl Eq for RunResult {}
impl Default for RunResult {
    fn default() -> Self {
        RunResult::Smc_RunResult(crate::ton::smc::runresult::RunResult::default())
    }
}
impl crate::BoxedSerialize for RunResult {
    fn serialize_boxed(&self) -> (crate::ConstructorNumber, &dyn crate::BareSerialize) {
        match self {
            &RunResult::Smc_RunResult(ref x) => (crate::ConstructorNumber(0x5444f3f3), x),
        }
    }
}
impl crate::BoxedDeserialize for RunResult {
    fn possible_constructors() -> Vec<crate::ConstructorNumber> {
        vec![crate::ConstructorNumber(0x5444f3f3)]
    }
    fn deserialize_boxed(
        _id: crate::ConstructorNumber,
        _de: &mut crate::Deserializer,
    ) -> crate::Result<Self> {
        match _id {
            crate::ConstructorNumber(0x5444f3f3) => Ok(RunResult::Smc_RunResult(
                _de.read_bare::<crate::ton::smc::runresult::RunResult>()?,
            )),
            id => _invalid_id!(id),
        }
    }
}
pub mod info;
pub mod methodid;
pub mod runresult;
