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
///https://reference.opcfoundation.org/v105/Core/docs/Part4/5.14.2/#5.14.2.2
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CreateSubscriptionRequest {
    pub request_header: opcua::types::request_header::RequestHeader,
    pub requested_publishing_interval: opcua::types::data_types::Duration,
    pub requested_lifetime_count: opcua::types::Counter,
    pub requested_max_keep_alive_count: opcua::types::Counter,
    pub max_notifications_per_publish: opcua::types::Counter,
    pub publishing_enabled: bool,
    pub priority: u8,
}
impl opcua::types::MessageInfo for CreateSubscriptionRequest {
    fn type_id(&self) -> opcua::types::ObjectId {
        opcua::types::ObjectId::CreateSubscriptionRequest_Encoding_DefaultBinary
    }
    fn json_type_id(&self) -> opcua::types::ObjectId {
        opcua::types::ObjectId::CreateSubscriptionRequest_Encoding_DefaultJson
    }
    fn xml_type_id(&self) -> opcua::types::ObjectId {
        opcua::types::ObjectId::CreateSubscriptionRequest_Encoding_DefaultXml
    }
    fn data_type_id(&self) -> opcua::types::DataTypeId {
        opcua::types::DataTypeId::CreateSubscriptionRequest
    }
}
