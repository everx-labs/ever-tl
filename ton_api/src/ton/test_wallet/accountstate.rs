use serde_derive::{Deserialize, Serialize};
#[derive(Debug, Default, Clone, PartialEq)]
#[doc = "TL-derived from `testWallet.accountState`\n\n```text\ntestWallet.accountState seqno:int32 = AccountState;\n```\n"]
pub struct AccountState {
    pub seqno: crate::ton::int32,
}
impl Eq for AccountState {}
impl crate::BareSerialize for AccountState {
    fn constructor(&self) -> crate::ConstructorNumber {
        crate::ConstructorNumber(0x8593d255)
    }
    fn serialize_bare(&self, _ser: &mut crate::Serializer) -> crate::Result<()> {
        let &AccountState { ref seqno } = self;
        _ser.write_bare::<crate::ton::int32>(seqno)?;
        Ok(())
    }
}
impl crate::BareDeserialize for AccountState {
    fn deserialize_bare(_de: &mut crate::Deserializer) -> crate::Result<Self> {
        {
            let seqno = _de.read_bare::<crate::ton::int32>()?;
            Ok(Self { seqno })
        }
    }
}
impl crate::IntoBoxed for AccountState {
    type Boxed = crate::ton::AccountState;
    fn into_boxed(self) -> crate::ton::AccountState {
        crate::ton::AccountState::TestWallet_AccountState(self)
    }
}
