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
///https://reference.opcfoundation.org/v105/Core/docs/Part18/4.4.2
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EndpointType {
    pub endpoint_url: opcua::types::string::UAString,
    pub security_mode: super::enums::MessageSecurityMode,
    pub security_policy_uri: opcua::types::string::UAString,
    pub transport_profile_uri: opcua::types::string::UAString,
}
impl opcua::types::MessageInfo for EndpointType {
    fn type_id(&self) -> opcua::types::ObjectId {
        opcua::types::ObjectId::EndpointType_Encoding_DefaultBinary
    }
    fn json_type_id(&self) -> opcua::types::ObjectId {
        opcua::types::ObjectId::EndpointType_Encoding_DefaultJson
    }
    fn xml_type_id(&self) -> opcua::types::ObjectId {
        opcua::types::ObjectId::EndpointType_Encoding_DefaultXml
    }
    fn data_type_id(&self) -> opcua::types::DataTypeId {
        opcua::types::DataTypeId::EndpointType
    }
}
