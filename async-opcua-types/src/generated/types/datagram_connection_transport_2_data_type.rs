// This file was autogenerated from schemas/1.05/Opc.Ua.NodeSet2.Services.xml by async-opcua-codegen
//
// DO NOT EDIT THIS FILE

// OPCUA for Rust
// SPDX-License-Identifier: MPL-2.0
// Copyright (C) 2017-2024 Adam Lock, Einar Omang
#[allow(unused)]
mod opcua {
    pub(super) use crate as types;
}
#[opcua::types::ua_encodable]
///https://reference.opcfoundation.org/v105/Core/docs/Part14/6.4.1/#6.4.1.2.7
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DatagramConnectionTransport2DataType {
    pub discovery_address: opcua::types::extension_object::ExtensionObject,
    pub discovery_announce_rate: u32,
    pub discovery_max_message_size: u32,
    pub qos_category: opcua::types::string::UAString,
    pub datagram_qos: Option<Vec<opcua::types::extension_object::ExtensionObject>>,
}
impl opcua::types::MessageInfo for DatagramConnectionTransport2DataType {
    fn type_id(&self) -> opcua::types::ObjectId {
        opcua::types::ObjectId::DatagramConnectionTransport2DataType_Encoding_DefaultBinary
    }
    fn json_type_id(&self) -> opcua::types::ObjectId {
        opcua::types::ObjectId::DatagramConnectionTransport2DataType_Encoding_DefaultJson
    }
    fn xml_type_id(&self) -> opcua::types::ObjectId {
        opcua::types::ObjectId::DatagramConnectionTransport2DataType_Encoding_DefaultXml
    }
    fn data_type_id(&self) -> opcua::types::DataTypeId {
        opcua::types::DataTypeId::DatagramConnectionTransport2DataType
    }
}
