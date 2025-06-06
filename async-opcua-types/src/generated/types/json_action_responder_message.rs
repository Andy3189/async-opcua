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
#[derive(Debug, Clone, PartialEq, Default)]
pub struct JsonActionResponderMessage {
    pub message_id: opcua::types::string::UAString,
    pub message_type: opcua::types::string::UAString,
    pub publisher_id: opcua::types::string::UAString,
    pub timestamp: opcua::types::data_types::UtcTime,
    pub connection: super::pub_sub_connection_data_type::PubSubConnectionDataType,
}
