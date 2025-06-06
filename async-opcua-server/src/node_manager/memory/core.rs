use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use chrono::Offset;
use hashbrown::HashMap;
use opcua_nodes::NodeType;

use crate::{
    address_space::{read_node_value, AddressSpace, CoreNamespace},
    diagnostics::NamespaceMetadata,
    load_method_args,
    node_manager::{
        MethodCall, MonitoredItemRef, MonitoredItemUpdateRef, NodeManagersRef, ParsedReadValueId,
        RequestContext, ServerContext, SyncSampler,
    },
    subscriptions::CreateMonitoredItem,
    ServerCapabilities, ServerStatusWrapper,
};
use opcua_core::{sync::RwLock, trace_lock};
use opcua_types::{
    DataValue, DateTime, ExtensionObject, IdType, Identifier, MethodId, MonitoringMode, NodeId,
    NumericRange, ObjectId, ReferenceTypeId, StatusCode, TimeZoneDataType, TimestampsToReturn,
    VariableId, Variant, VariantScalarTypeId, VariantTypeId,
};

use super::{InMemoryNodeManager, InMemoryNodeManagerImpl, InMemoryNodeManagerImplBuilder};

/// Node manager impl for the core namespace.
pub struct CoreNodeManagerImpl {
    sampler: SyncSampler,
    node_managers: NodeManagersRef,
    status: Arc<ServerStatusWrapper>,
}

/// Node manager for the core namespace.
pub type CoreNodeManager = InMemoryNodeManager<CoreNodeManagerImpl>;

/// Builder for the [CoreNodeManager].
pub struct CoreNodeManagerBuilder;

impl InMemoryNodeManagerImplBuilder for CoreNodeManagerBuilder {
    type Impl = CoreNodeManagerImpl;

    fn build(self, context: ServerContext, address_space: &mut AddressSpace) -> Self::Impl {
        {
            let mut type_tree = context.type_tree.write();
            address_space.import_node_set(&CoreNamespace, type_tree.namespaces_mut());
        }

        CoreNodeManagerImpl::new(context.node_managers.clone(), context.status.clone())
    }
}

/*
The core node manager serves as an example for how you can create a simple
node manager based on the in-memory node manager.

In this case the data is largely static, so all we need to really
implement is Read, leaving the responsibility for notifying any subscriptions
of changes to these to the one doing the modifying.
*/

#[async_trait]
impl InMemoryNodeManagerImpl for CoreNodeManagerImpl {
    async fn init(&self, address_space: &mut AddressSpace, context: ServerContext) {
        self.add_aggregates(address_space, &context.info.capabilities);
        let interval = context
            .info
            .config
            .limits
            .subscriptions
            .min_sampling_interval_ms
            .floor() as u64;
        let sampler_interval = if interval > 0 { interval } else { 100 };
        self.sampler.run(
            Duration::from_millis(sampler_interval),
            context.subscriptions.clone(),
        );
        // Some core methods should be generally executable
        Self::set_method_executable(address_space, MethodId::Server_GetMonitoredItems);
        Self::set_method_executable(address_space, MethodId::Server_ResendData);
    }

    fn namespaces(&self) -> Vec<NamespaceMetadata> {
        vec![NamespaceMetadata {
            // If necessary we could read this from the address space here,
            // but I don't think we need to, the diagnostics node manager
            // has an exception for the base namespace.
            is_namespace_subset: Some(false),
            namespace_publication_date: None,
            namespace_version: None,
            namespace_uri: "http://opcfoundation.org/UA/".to_owned(),
            static_node_id_types: Some(vec![IdType::Numeric]),
            namespace_index: 0,
            ..Default::default()
        }]
    }

    fn name(&self) -> &str {
        "core"
    }

    async fn read_values(
        &self,
        context: &RequestContext,
        address_space: &RwLock<AddressSpace>,
        nodes: &[&ParsedReadValueId],
        max_age: f64,
        timestamps_to_return: TimestampsToReturn,
    ) -> Vec<DataValue> {
        let address_space = address_space.read();

        nodes
            .iter()
            .map(|n| {
                self.read_node_value(context, &address_space, n, max_age, timestamps_to_return)
            })
            .collect()
    }

    async fn call(
        &self,
        context: &RequestContext,
        _address_space: &RwLock<AddressSpace>,
        methods_to_call: &mut [&mut &mut MethodCall],
    ) -> Result<(), StatusCode> {
        for method in methods_to_call {
            if let Err(e) = self.call_builtin_method(method, context) {
                method.set_status(e);
            }
        }
        Ok(())
    }

    async fn create_value_monitored_items(
        &self,
        context: &RequestContext,
        address_space: &RwLock<AddressSpace>,
        items: &mut [&mut &mut CreateMonitoredItem],
    ) {
        let address_space = address_space.read();
        for node in items {
            let value = self.read_node_value(
                context,
                &address_space,
                node.item_to_monitor(),
                0.0,
                node.timestamps_to_return(),
            );
            if value.status() == StatusCode::BadUserAccessDenied {
                node.set_status(StatusCode::BadUserAccessDenied);
                continue;
            }
            if value.status() != StatusCode::BadAttributeIdInvalid {
                node.set_initial_value(value);
            }
            node.set_status(StatusCode::Good);

            if let Some(var_id) = self.status.get_managed_id(&node.item_to_monitor().node_id) {
                self.status.subscribe_to_component(
                    var_id,
                    node.monitoring_mode(),
                    node.handle(),
                    Duration::from_millis(node.sampling_interval() as u64),
                );
            } else if self.is_internal_sampled(&node.item_to_monitor().node_id, context) {
                if let Err(e) = self.add_internal_sampler(node, context) {
                    node.set_status(e);
                }
            }
        }
    }

    async fn set_monitoring_mode(
        &self,
        _context: &RequestContext,
        mode: MonitoringMode,
        items: &[&MonitoredItemRef],
    ) {
        for item in items {
            if self.status.get_managed_id(item.node_id()).is_some() {
                self.status.sampler().set_sampler_mode(
                    item.node_id(),
                    item.attribute(),
                    item.handle(),
                    mode,
                );
            }
        }
    }

    async fn modify_monitored_items(
        &self,
        _context: &RequestContext,
        items: &[&MonitoredItemUpdateRef],
    ) {
        for item in items {
            if self.status.get_managed_id(item.node_id()).is_some() {
                self.status.sampler().update_sampler(
                    item.node_id(),
                    item.attribute(),
                    item.handle(),
                    Duration::from_millis(item.update().revised_sampling_interval as u64),
                );
            }
        }
    }

    async fn delete_monitored_items(&self, _context: &RequestContext, items: &[&MonitoredItemRef]) {
        for item in items {
            if self.status.get_managed_id(item.node_id()).is_some() {
                self.status.sampler().remove_sampler(
                    item.node_id(),
                    item.attribute(),
                    item.handle(),
                );
            }
        }
    }
}

impl CoreNodeManagerImpl {
    pub(super) fn new(node_managers: NodeManagersRef, status: Arc<ServerStatusWrapper>) -> Self {
        Self {
            sampler: SyncSampler::new(),
            status,
            node_managers,
        }
    }

    fn read_node_value(
        &self,
        context: &RequestContext,
        address_space: &AddressSpace,
        node_to_read: &ParsedReadValueId,
        max_age: f64,
        timestamps_to_return: TimestampsToReturn,
    ) -> DataValue {
        let mut result_value = DataValue::null();
        // Check that the read is permitted.
        let node = match address_space.validate_node_read(context, node_to_read) {
            Ok(n) => n,
            Err(e) => {
                result_value.status = Some(e);
                return result_value;
            }
        };
        // Try to read a special value, that is obtained from somewhere else.
        // A custom node manager might read this from some device, or get them
        // in some other way.

        // In this case, the values are largely read from configuration.
        if let Some(v) = self.read_server_value(context, node_to_read) {
            v
        } else {
            // If it can't be found, read it from the node hierarchy.
            read_node_value(node, context, node_to_read, max_age, timestamps_to_return)
        }
    }

    fn get_variable_id(&self, node: &NodeId) -> Option<VariableId> {
        if node.namespace != 0 {
            return None;
        }
        let Identifier::Numeric(identifier) = node.identifier else {
            return None;
        };
        VariableId::try_from(identifier).ok()
    }

    fn is_internal_sampled(&self, node: &NodeId, context: &RequestContext) -> bool {
        let Some(variable_id) = self.get_variable_id(node) else {
            return false;
        };

        context.info.diagnostics.is_mapped(variable_id)
    }

    fn add_internal_sampler(
        &self,
        monitored_item: &mut CreateMonitoredItem,
        context: &RequestContext,
    ) -> Result<(), StatusCode> {
        let Some(var_id) = self.get_variable_id(&monitored_item.item_to_monitor().node_id) else {
            return Err(StatusCode::BadNodeIdUnknown);
        };

        if context.info.diagnostics.is_mapped(var_id) {
            let info = context.info.clone();
            self.sampler.add_sampler(
                monitored_item.item_to_monitor().node_id.clone(),
                monitored_item.item_to_monitor().attribute_id,
                move || info.diagnostics.get(var_id),
                monitored_item.monitoring_mode(),
                monitored_item.handle(),
                Duration::from_millis(monitored_item.sampling_interval() as u64),
            );
            Ok(())
        } else {
            Err(StatusCode::BadNodeIdUnknown)
        }
    }

    fn read_server_value(
        &self,
        context: &RequestContext,
        node: &ParsedReadValueId,
    ) -> Option<DataValue> {
        let var_id = self.get_variable_id(&node.node_id)?;

        let limits = &context.info.config.limits;
        let hist_cap = &context.info.capabilities.history;

        let v: Variant = match var_id {
            VariableId::Server_ServerCapabilities_MaxArrayLength => {
                (limits.max_array_length as u32).into()
            }
            VariableId::Server_ServerCapabilities_MaxBrowseContinuationPoints => {
                (limits.max_browse_continuation_points as u16).into()
            }
            VariableId::Server_ServerCapabilities_MaxByteStringLength => {
                (limits.max_byte_string_length as u32).into()
            }
            VariableId::Server_ServerCapabilities_MaxHistoryContinuationPoints => {
                (limits.max_history_continuation_points as u16).into()
            }
            VariableId::Server_ServerCapabilities_MaxQueryContinuationPoints => {
                (limits.max_query_continuation_points as u16).into()
            }
            VariableId::Server_ServerCapabilities_MaxStringLength => {
                (limits.max_string_length as u32).into()
            }
            VariableId::Server_ServerCapabilities_MinSupportedSampleRate => {
                (limits.subscriptions.min_sampling_interval_ms as u32).into()
            }
            VariableId::Server_ServerCapabilities_OperationLimits_MaxMonitoredItemsPerCall => {
                (limits.operational.max_monitored_items_per_call as u32).into()
            }
            VariableId::Server_ServerCapabilities_OperationLimits_MaxNodesPerBrowse => {
                (limits.operational.max_nodes_per_browse as u32).into()
            }
            VariableId::Server_ServerCapabilities_OperationLimits_MaxNodesPerHistoryReadData => {
                (limits.operational.max_nodes_per_history_read_data as u32).into()
            }
            VariableId::Server_ServerCapabilities_OperationLimits_MaxNodesPerHistoryReadEvents => {
                (limits.operational.max_nodes_per_history_read_events as u32).into()
            }
            VariableId::Server_ServerCapabilities_OperationLimits_MaxNodesPerHistoryUpdateData => {
                (limits.operational.max_nodes_per_history_update as u32).into()
            }
            VariableId::Server_ServerCapabilities_OperationLimits_MaxNodesPerHistoryUpdateEvents => {
                (limits.operational.max_nodes_per_history_update as u32).into()
            }
            VariableId::Server_ServerCapabilities_OperationLimits_MaxNodesPerMethodCall => {
                (limits.operational.max_nodes_per_method_call as u32).into()
            }
            VariableId::Server_ServerCapabilities_OperationLimits_MaxNodesPerNodeManagement => {
                (limits.operational.max_nodes_per_node_management as u32).into()
            }
            VariableId::Server_ServerCapabilities_OperationLimits_MaxNodesPerRead => {
                (limits.operational.max_nodes_per_read as u32).into()
            }
            VariableId::Server_ServerCapabilities_OperationLimits_MaxNodesPerRegisterNodes => {
                (limits.operational.max_nodes_per_register_nodes as u32).into()
            }
            VariableId::Server_ServerCapabilities_OperationLimits_MaxNodesPerTranslateBrowsePathsToNodeIds => {
                (limits.operational.max_nodes_per_translate_browse_paths_to_node_ids as u32).into()
            }
            VariableId::Server_ServerCapabilities_OperationLimits_MaxNodesPerWrite => {
                (limits.operational.max_nodes_per_write as u32).into()
            }
            VariableId::Server_ServerCapabilities_ServerProfileArray => {
                context.info.capabilities.profiles.clone().into()
            }

            // History capabilities
            VariableId::HistoryServerCapabilities_AccessHistoryDataCapability => {
                hist_cap.access_history_data.into()
            }
            VariableId::HistoryServerCapabilities_AccessHistoryEventsCapability => {
                hist_cap.access_history_events.into()
            }
            VariableId::HistoryServerCapabilities_DeleteAtTimeCapability => {
                hist_cap.delete_at_time.into()
            }
            VariableId::HistoryServerCapabilities_DeleteEventCapability => {
                hist_cap.delete_event.into()
            }
            VariableId::HistoryServerCapabilities_DeleteRawCapability => {
                hist_cap.delete_raw.into()
            }
            VariableId::HistoryServerCapabilities_InsertAnnotationCapability => {
                hist_cap.insert_annotation.into()
            }
            VariableId::HistoryServerCapabilities_InsertDataCapability => {
                hist_cap.insert_data.into()
            }
            VariableId::HistoryServerCapabilities_InsertEventCapability => {
                hist_cap.insert_event.into()
            }
            VariableId::HistoryServerCapabilities_MaxReturnDataValues => {
                hist_cap.max_return_data_values.into()
            }
            VariableId::HistoryServerCapabilities_MaxReturnEventValues => {
                hist_cap.max_return_event_values.into()
            }
            VariableId::HistoryServerCapabilities_ReplaceDataCapability => {
                hist_cap.replace_data.into()
            }
            VariableId::HistoryServerCapabilities_ReplaceEventCapability => {
                hist_cap.replace_event.into()
            }
            VariableId::HistoryServerCapabilities_ServerTimestampSupported => {
                hist_cap.server_timestamp_supported.into()
            }
            VariableId::HistoryServerCapabilities_UpdateDataCapability => {
                hist_cap.update_data.into()
            }
            VariableId::HistoryServerCapabilities_UpdateEventCapability => {
                hist_cap.update_event.into()
            }

            // Misc server status
            VariableId::Server_ServiceLevel => {
                context.info.service_level.load(std::sync::atomic::Ordering::Relaxed).into()
            }
            VariableId::Server_LocalTime => {
                let offset = chrono::Local::now().offset().fix().local_minus_utc() / 60;
                ExtensionObject::from_message(TimeZoneDataType {
                    offset: offset.try_into().ok()?,
                    // TODO: Figure out how to set this. Chrono does not provide a way to
                    // tell whether daylight savings is in effect for the local time zone.
                    daylight_saving_in_offset: false,
                }).into()
            }

            // ServerStatus
            VariableId::Server_ServerStatus => {
                self.status.full_status_obj().into()
            }
            VariableId::Server_ServerStatus_BuildInfo => {
                ExtensionObject::from_message(self.status.build_info()).into()
            }
            VariableId::Server_ServerStatus_BuildInfo_BuildDate => {
                self.status.build_info().build_date.into()
            }
            VariableId::Server_ServerStatus_BuildInfo_BuildNumber => {
                self.status.build_info().build_number.into()
            }
            VariableId::Server_ServerStatus_BuildInfo_ManufacturerName => {
                self.status.build_info().manufacturer_name.into()
            }
            VariableId::Server_ServerStatus_BuildInfo_ProductName => {
                self.status.build_info().product_name.into()
            }
            VariableId::Server_ServerStatus_BuildInfo_ProductUri => {
                self.status.build_info().product_uri.into()
            }
            VariableId::Server_ServerStatus_BuildInfo_SoftwareVersion => {
                self.status.build_info().software_version.into()
            }
            VariableId::Server_ServerStatus_CurrentTime => {
                DateTime::now().into()
            }
            VariableId::Server_ServerStatus_SecondsTillShutdown => {
                match self.status.seconds_till_shutdown() {
                    Some(x) => x.into(),
                    None => Variant::Empty
                }
            }
            VariableId::Server_ServerStatus_ShutdownReason => {
                self.status.shutdown_reason().into()
            }
            VariableId::Server_ServerStatus_StartTime => {
                self.status.start_time().into()
            }
            VariableId::Server_ServerStatus_State => {
                (self.status.state() as i32).into()
            }

            VariableId::Server_NamespaceArray => {
                // This actually calls into other node managers to obtain the value, in fact
                // it calls into _this_ node manager as well.
                // Be careful to avoid holding exclusive locks in a way that causes a deadlock
                // when doing this. Here we hold a read lock on the address space,
                // but in this case it doesn't matter.
                let nss: HashMap<_, _> = self.node_managers.iter().flat_map(|n| n.namespaces_for_user(context)).map(|ns| (ns.namespace_index, ns.namespace_uri)).collect();
                // Make sure that holes are filled with empty strings, so that the
                // namespace array actually has correct indices.
                let &max = nss.keys().max()?;
                let namespaces: Vec<_> = (0..(max + 1)).map(|idx| nss.get(&idx).cloned().unwrap_or_default()).collect();
                namespaces.into()
            }

            r if context.info.diagnostics.is_mapped(r) => {
                let perms = context.info.authenticator.core_permissions(&context.token);
                if !perms.read_diagnostics {
                    return Some(DataValue::new_now_status(Variant::Empty, StatusCode::BadUserAccessDenied));
                } else {
                    return Some(context.info.diagnostics.get(r).unwrap_or_default())
                }
            }

            _ => return None,

        };

        let v = if !matches!(node.index_range, NumericRange::None) {
            match v.range_of(&node.index_range) {
                Ok(v) => v,
                Err(e) => {
                    return Some(DataValue {
                        value: None,
                        status: Some(e),
                        ..Default::default()
                    })
                }
            }
        } else {
            v
        };

        Some(DataValue {
            value: Some(v),
            status: Some(StatusCode::Good),
            source_timestamp: Some(**context.info.start_time.load()),
            server_timestamp: Some(**context.info.start_time.load()),
            ..Default::default()
        })
    }

    fn add_aggregates(&self, address_space: &mut AddressSpace, capabilities: &ServerCapabilities) {
        for aggregate in &capabilities.history.aggregates {
            address_space.insert_reference(
                &ObjectId::HistoryServerCapabilities_AggregateFunctions.into(),
                aggregate,
                ReferenceTypeId::Organizes,
            )
        }
    }

    fn set_method_executable(address_space: &mut AddressSpace, method: MethodId) {
        let Some(NodeType::Method(m)) = address_space.find_mut(method) else {
            return;
        };
        m.set_executable(true);
        m.set_user_executable(true);
    }

    fn call_builtin_method(
        &self,
        call: &mut MethodCall,
        context: &RequestContext,
    ) -> Result<(), StatusCode> {
        let Ok(id) = call.method_id().as_method_id() else {
            return Ok(());
        };

        match id {
            MethodId::Server_GetMonitoredItems => {
                let id = load_method_args!(call, UInt32)?;
                let subs = context
                    .subscriptions
                    .get_session_subscriptions(context.session_id)
                    .ok_or(StatusCode::BadSessionIdInvalid)?;
                let subs = trace_lock!(subs);
                let sub = subs.get(id).ok_or(StatusCode::BadSubscriptionIdInvalid)?;
                let (ids, handles): (Vec<_>, Vec<_>) =
                    sub.items().map(|i| (i.id(), i.client_handle())).unzip();
                call.set_outputs(vec![ids.into(), handles.into()]);
                call.set_status(StatusCode::Good);
            }
            MethodId::Server_ResendData => {
                let id = load_method_args!(call, UInt32)?;
                let subs = context
                    .subscriptions
                    .get_session_subscriptions(context.session_id)
                    .ok_or(StatusCode::BadSessionIdInvalid)?;
                let mut subs = trace_lock!(subs);
                let sub = subs
                    .get_mut(id)
                    .ok_or(StatusCode::BadSubscriptionIdInvalid)?;
                sub.set_resend_data();
                call.set_status(StatusCode::Good);
            }
            _ => return Err(StatusCode::BadNotSupported),
        }
        Ok(())
    }
}
