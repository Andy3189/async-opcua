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
///https://reference.opcfoundation.org/v105/Core/docs/Part14/6.2.12/#6.2.12.2
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SecurityGroupDataType {
    pub name: opcua::types::string::UAString,
    pub security_group_folder: Option<Vec<opcua::types::string::UAString>>,
    pub key_lifetime: opcua::types::data_types::Duration,
    pub security_policy_uri: opcua::types::string::UAString,
    pub max_future_key_count: u32,
    pub max_past_key_count: u32,
    pub security_group_id: opcua::types::string::UAString,
    pub role_permissions: Option<Vec<super::role_permission_type::RolePermissionType>>,
    pub group_properties: Option<Vec<super::key_value_pair::KeyValuePair>>,
}
impl opcua::types::MessageInfo for SecurityGroupDataType {
    fn type_id(&self) -> opcua::types::ObjectId {
        opcua::types::ObjectId::SecurityGroupDataType_Encoding_DefaultBinary
    }
    fn json_type_id(&self) -> opcua::types::ObjectId {
        opcua::types::ObjectId::SecurityGroupDataType_Encoding_DefaultJson
    }
    fn xml_type_id(&self) -> opcua::types::ObjectId {
        opcua::types::ObjectId::SecurityGroupDataType_Encoding_DefaultXml
    }
    fn data_type_id(&self) -> opcua::types::DataTypeId {
        opcua::types::DataTypeId::SecurityGroupDataType
    }
}
