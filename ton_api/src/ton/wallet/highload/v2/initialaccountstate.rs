use serde_derive::{Deserialize, Serialize};
#[derive(Debug, Default, Clone, PartialEq)]
#[doc = "TL-derived from `wallet.highload.v2.initialAccountState`\n\n```text\nwallet.highload.v2.initialAccountState public_key:string wallet_id:int64 = InitialAccountState;\n```\n"]
pub struct InitialAccountState {
    pub public_key: crate::ton::string,
    pub wallet_id: crate::ton::int64,
}
impl Eq for InitialAccountState {}
impl crate::BareSerialize for InitialAccountState {
    fn constructor(&self) -> crate::ConstructorNumber {
        crate::ConstructorNumber(0x75347929)
    }
    fn serialize_bare(&self, _ser: &mut crate::Serializer) -> crate::Result<()> {
        let &InitialAccountState {
            ref public_key,
            ref wallet_id,
        } = self;
        _ser.write_bare::<crate::ton::string>(public_key)?;
        _ser.write_bare::<crate::ton::int64>(wallet_id)?;
        Ok(())
    }
}
impl crate::BareDeserialize for InitialAccountState {
    fn deserialize_bare(_de: &mut crate::Deserializer) -> crate::Result<Self> {
        {
            let public_key = _de.read_bare::<crate::ton::string>()?;
            let wallet_id = _de.read_bare::<crate::ton::int64>()?;
            Ok(Self {
                public_key,
                wallet_id,
            })
        }
    }
}
impl crate::IntoBoxed for InitialAccountState {
    type Boxed = crate::ton::InitialAccountState;
    fn into_boxed(self) -> crate::ton::InitialAccountState {
        crate::ton::InitialAccountState::Wallet_Highload_V2_InitialAccountState(self)
    }
}
