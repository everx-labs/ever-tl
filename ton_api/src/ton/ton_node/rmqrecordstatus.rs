use serde_derive::{Deserialize, Serialize};
#[derive(Debug, Default, Clone, PartialEq)]
#[doc = "TL-derived from `tonNode.rmqAccepted`\n\n```text\ntonNode.rmqAccepted block_id:tonNode.blockIdExt = tonNode.RmqRecordStatus;\n```\n"]
pub struct RmqAccepted {
    pub block_id: crate::ton::ton_node::blockidext::BlockIdExt,
}
impl Eq for RmqAccepted {}
impl crate::BareSerialize for RmqAccepted {
    fn constructor(&self) -> crate::ConstructorNumber {
        crate::ConstructorNumber(0x4d81c0b1)
    }
    fn serialize_bare(&self, _ser: &mut crate::Serializer) -> crate::Result<()> {
        let &RmqAccepted { ref block_id } = self;
        _ser.write_bare::<crate::ton::ton_node::blockidext::BlockIdExt>(block_id)?;
        Ok(())
    }
}
impl crate::BareDeserialize for RmqAccepted {
    fn deserialize_bare(_de: &mut crate::Deserializer) -> crate::Result<Self> {
        {
            let block_id = _de.read_bare::<crate::ton::ton_node::blockidext::BlockIdExt>()?;
            Ok(Self { block_id })
        }
    }
}
impl crate::IntoBoxed for RmqAccepted {
    type Boxed = crate::ton::ton_node::RmqRecordStatus;
    fn into_boxed(self) -> crate::ton::ton_node::RmqRecordStatus {
        crate::ton::ton_node::RmqRecordStatus::TonNode_RmqAccepted(self)
    }
}
#[derive(Debug, Default, Clone, PartialEq)]
#[doc = "TL-derived from `tonNode.rmqRejected`\n\n```text\ntonNode.rmqRejected block_id:tonNode.blockIdExt error:string = tonNode.RmqRecordStatus;\n```\n"]
pub struct RmqRejected {
    pub block_id: crate::ton::ton_node::blockidext::BlockIdExt,
    pub error: crate::ton::string,
}
impl Eq for RmqRejected {}
impl crate::BareSerialize for RmqRejected {
    fn constructor(&self) -> crate::ConstructorNumber {
        crate::ConstructorNumber(0x6349b79f)
    }
    fn serialize_bare(&self, _ser: &mut crate::Serializer) -> crate::Result<()> {
        let &RmqRejected {
            ref block_id,
            ref error,
        } = self;
        _ser.write_bare::<crate::ton::ton_node::blockidext::BlockIdExt>(block_id)?;
        _ser.write_bare::<crate::ton::string>(error)?;
        Ok(())
    }
}
impl crate::BareDeserialize for RmqRejected {
    fn deserialize_bare(_de: &mut crate::Deserializer) -> crate::Result<Self> {
        {
            let block_id = _de.read_bare::<crate::ton::ton_node::blockidext::BlockIdExt>()?;
            let error = _de.read_bare::<crate::ton::string>()?;
            Ok(Self { block_id, error })
        }
    }
}
impl crate::IntoBoxed for RmqRejected {
    type Boxed = crate::ton::ton_node::RmqRecordStatus;
    fn into_boxed(self) -> crate::ton::ton_node::RmqRecordStatus {
        crate::ton::ton_node::RmqRecordStatus::TonNode_RmqRejected(self)
    }
}
