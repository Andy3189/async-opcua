// OPCUA for Rust
// SPDX-License-Identifier: MPL-2.0
// Copyright (C) 2017-2024 Adam Lock

//! Contains the implementation of `ObjectType` and `ObjectTypeBuilder`.

use opcua_types::{
    AttributeId, AttributesMask, DataEncoding, DataValue, NumericRange, ObjectTypeAttributes,
    StatusCode, TimestampsToReturn, Variant,
};
use tracing::error;

use crate::FromAttributesError;

use super::{base::Base, node::Node, node::NodeBase};

node_builder_impl!(ObjectTypeBuilder, ObjectType);

node_builder_impl_generates_event!(ObjectTypeBuilder);
node_builder_impl_subtype!(ObjectTypeBuilder);
node_builder_impl_component_of!(ObjectTypeBuilder);
node_builder_impl_property_of!(ObjectTypeBuilder);

impl ObjectTypeBuilder {
    /// Set whether the object type is abstract, meaning
    /// it cannot be used by types in the instance hierarchy.
    pub fn is_abstract(mut self, is_abstract: bool) -> Self {
        self.node.set_is_abstract(is_abstract);
        self
    }

    /// Set the object type write mask.
    pub fn write_mask(mut self, write_mask: WriteMask) -> Self {
        self.node.set_write_mask(write_mask);
        self
    }
}

/// An `ObjectType` is a type of node within the `AddressSpace`.
#[derive(Debug)]
pub struct ObjectType {
    pub(super) base: Base,
    pub(super) is_abstract: bool,
}

impl Default for ObjectType {
    fn default() -> Self {
        Self {
            base: Base::new(NodeClass::ObjectType, &NodeId::null(), "", ""),
            is_abstract: false,
        }
    }
}

node_base_impl!(ObjectType);

impl Node for ObjectType {
    fn get_attribute_max_age(
        &self,
        timestamps_to_return: TimestampsToReturn,
        attribute_id: AttributeId,
        index_range: &NumericRange,
        data_encoding: &DataEncoding,
        max_age: f64,
    ) -> Option<DataValue> {
        match attribute_id {
            AttributeId::IsAbstract => Some(self.is_abstract().into()),
            _ => self.base.get_attribute_max_age(
                timestamps_to_return,
                attribute_id,
                index_range,
                data_encoding,
                max_age,
            ),
        }
    }

    fn set_attribute(
        &mut self,
        attribute_id: AttributeId,
        value: Variant,
    ) -> Result<(), StatusCode> {
        match attribute_id {
            AttributeId::IsAbstract => {
                if let Variant::Boolean(v) = value {
                    self.set_is_abstract(v);
                    Ok(())
                } else {
                    Err(StatusCode::BadTypeMismatch)
                }
            }
            _ => self.base.set_attribute(attribute_id, value),
        }
    }
}

impl ObjectType {
    /// Create a new object type.
    pub fn new(
        node_id: &NodeId,
        browse_name: impl Into<QualifiedName>,
        display_name: impl Into<LocalizedText>,
        is_abstract: bool,
    ) -> ObjectType {
        ObjectType {
            base: Base::new(NodeClass::ObjectType, node_id, browse_name, display_name),
            is_abstract,
        }
    }

    /// Create a new object type with all attributes, may change if
    /// new attributes are added to the OPC-UA standard.
    pub fn new_full(base: Base, is_abstract: bool) -> Self {
        Self { base, is_abstract }
    }

    /// Create a new object type from [ObjectTypeAttributes].
    pub fn from_attributes(
        node_id: &NodeId,
        browse_name: impl Into<QualifiedName>,
        attributes: ObjectTypeAttributes,
    ) -> Result<Self, FromAttributesError> {
        let mandatory_attributes = AttributesMask::DISPLAY_NAME | AttributesMask::IS_ABSTRACT;
        let mask = AttributesMask::from_bits(attributes.specified_attributes)
            .ok_or(FromAttributesError::InvalidMask)?;
        if mask.contains(mandatory_attributes) {
            let mut node = Self::new(
                node_id,
                browse_name,
                attributes.display_name,
                attributes.is_abstract,
            );
            if mask.contains(AttributesMask::DESCRIPTION) {
                node.set_description(attributes.description);
            }
            if mask.contains(AttributesMask::WRITE_MASK) {
                node.set_write_mask(WriteMask::from_bits_truncate(attributes.write_mask));
            }
            if mask.contains(AttributesMask::USER_WRITE_MASK) {
                node.set_user_write_mask(WriteMask::from_bits_truncate(attributes.user_write_mask));
            }
            Ok(node)
        } else {
            error!("ObjectType cannot be created from attributes - missing mandatory values");
            Err(FromAttributesError::MissingMandatoryValues)
        }
    }

    /// Get whether this object type is valid.
    pub fn is_valid(&self) -> bool {
        self.base.is_valid()
    }

    /// Get the `IsAbstract` attribute for this object type.
    pub fn is_abstract(&self) -> bool {
        self.is_abstract
    }

    /// Set the `IsAbstract` attribute for this object type.
    pub fn set_is_abstract(&mut self, is_abstract: bool) {
        self.is_abstract = is_abstract;
    }
}
