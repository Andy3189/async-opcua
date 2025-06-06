// OPCUA for Rust
// SPDX-License-Identifier: MPL-2.0
// Copyright (C) 2017-2024 Adam Lock

//! Contains the implementation of `DataValue`.

use std::io::{Read, Write};

use crate::{
    byte_string::ByteString, date_time::*, guid::Guid, localized_text::LocalizedText,
    node_id::NodeId, qualified_name::QualifiedName, status_code::StatusCode, string::UAString,
    variant::Variant, BinaryDecodable, BinaryEncodable, Context, EncodingResult,
    TimestampsToReturn,
};
use bitflags::bitflags;

bitflags! {
    struct DataValueFlags: u8 {
        /// False if the Value is Null.
        const HAS_VALUE = 0x1;
        /// False if the StatusCode is Good.
        const HAS_STATUS = 0x2;
        /// False if the Source Timestamp is DateTime.MinValue.
        const HAS_SOURCE_TIMESTAMP = 0x4;
        /// False if the Server Timestamp is DateTime.MinValue.
        const HAS_SERVER_TIMESTAMP = 0x8;
        /// False if the Source Picoseconds is 0.
        const HAS_SOURCE_PICOSECONDS = 0x10;
        /// False if the Server Picoseconds is 0.
        const HAS_SERVER_PICOSECONDS = 0x20;
    }
}

#[allow(unused)]
mod opcua {
    pub(super) use crate as types;
}

/// A data value is a value of a variable in the OPC UA server and contains information about its
/// value, status and change timestamps.
#[derive(Debug, Clone, PartialEq, crate::UaNullable)]
#[cfg_attr(
    feature = "json",
    derive(opcua_macros::JsonEncodable, opcua_macros::JsonDecodable)
)]
pub struct DataValue {
    /// The value. BaseDataType
    /// Not present if the Value bit in the EncodingMask is False.
    pub value: Option<Variant>,
    /// The status associated with the value.
    /// Not present if the StatusCode bit in the EncodingMask is False
    pub status: Option<StatusCode>,
    /// The source timestamp associated with the value.
    /// Not present if the SourceTimestamp bit in the EncodingMask is False.
    pub source_timestamp: Option<DateTime>,
    /// The number of 10 picosecond intervals for the SourceTimestamp.
    /// Not present if the SourcePicoSeconds bit in the EncodingMask is False.
    /// If the source timestamp is missing the picoseconds are ignored.
    pub source_picoseconds: Option<u16>,
    /// The Server timestamp associated with the value.
    /// Not present if the ServerTimestamp bit in the EncodingMask is False.
    pub server_timestamp: Option<DateTime>,
    /// The number of 10 picosecond intervals for the ServerTimestamp.
    /// Not present if the ServerPicoSeconds bit in the EncodingMask is False.
    /// If the Server timestamp is missing the picoseconds are ignored.
    pub server_picoseconds: Option<u16>,
}

// For some spectacularly dumb reason Status is different in JSON and XML.
// It is named "StatusCode" in XML (5.3.1.18) and "Status" in JSON (5.4.2.18), the _only_ place
// where this is the case on a struct. So we have to implement XML manually.
#[cfg(feature = "xml")]
mod xml {
    use super::DataValue;

    impl crate::xml::XmlEncodable for DataValue {
        fn encode(
            &self,
            stream: &mut crate::xml::XmlStreamWriter<&mut dyn std::io::Write>,
            ctx: &crate::Context<'_>,
        ) -> crate::EncodingResult<()> {
            use crate::xml::XmlWriteExt;
            stream.encode_child("Value", &self.value, ctx)?;
            stream.encode_child("StatusCode", &self.status, ctx)?;
            stream.encode_child("SourceTimestamp", &self.source_timestamp, ctx)?;
            stream.encode_child("SourcePicoseconds", &self.source_picoseconds, ctx)?;
            stream.encode_child("ServerTimestamp", &self.server_timestamp, ctx)?;
            stream.encode_child("ServerPicoseconds", &self.server_picoseconds, ctx)?;
            Ok(())
        }
    }
    impl crate::xml::XmlDecodable for DataValue {
        fn decode(
            stream: &mut crate::xml::XmlStreamReader<&mut dyn std::io::Read>,
            ctx: &crate::Context<'_>,
        ) -> crate::EncodingResult<Self> {
            use crate::xml::XmlReadExt;
            let mut value = None;
            let mut status = None;
            let mut source_timestamp = None;
            let mut source_picoseconds = None;
            let mut server_timestamp = None;
            let mut server_picoseconds = None;
            stream.iter_children(
                |__key, stream, ctx| {
                    match __key.as_str() {
                        "Value" => {
                            value = Some(crate::xml::XmlDecodable::decode(stream, ctx)?);
                        }
                        "StatusCode" => {
                            status = Some(crate::xml::XmlDecodable::decode(stream, ctx)?);
                        }
                        "SourceTimestamp" => {
                            source_timestamp = Some(crate::xml::XmlDecodable::decode(stream, ctx)?);
                        }
                        "SourcePicoseconds" => {
                            source_picoseconds =
                                Some(crate::xml::XmlDecodable::decode(stream, ctx)?);
                        }
                        "ServerTimestamp" => {
                            server_timestamp = Some(crate::xml::XmlDecodable::decode(stream, ctx)?);
                        }
                        "ServerPicoseconds" => {
                            server_picoseconds =
                                Some(crate::xml::XmlDecodable::decode(stream, ctx)?);
                        }
                        _ => {
                            stream.skip_value()?;
                        }
                    }
                    Ok(())
                },
                ctx,
            )?;
            Ok(Self {
                value: value.unwrap_or_default(),
                status: status.unwrap_or_default(),
                source_timestamp: source_timestamp.unwrap_or_default(),
                source_picoseconds: source_picoseconds.unwrap_or_default(),
                server_timestamp: server_timestamp.unwrap_or_default(),
                server_picoseconds: server_picoseconds.unwrap_or_default(),
            })
        }
    }
    impl crate::xml::XmlType for DataValue {
        const TAG: &'static str = "DataValue";
    }
}

impl BinaryEncodable for DataValue {
    fn byte_len(&self, ctx: &opcua::types::Context<'_>) -> usize {
        let mut size = 1;
        let encoding_mask = self.encoding_mask();
        if encoding_mask.contains(DataValueFlags::HAS_VALUE) {
            size += self.value.as_ref().unwrap().byte_len(ctx);
        }
        if encoding_mask.contains(DataValueFlags::HAS_STATUS) {
            size += self.status.as_ref().unwrap().byte_len(ctx);
        }
        if encoding_mask.contains(DataValueFlags::HAS_SOURCE_TIMESTAMP) {
            size += self.source_timestamp.as_ref().unwrap().byte_len(ctx);
            if encoding_mask.contains(DataValueFlags::HAS_SOURCE_PICOSECONDS) {
                size += self.source_picoseconds.as_ref().unwrap().byte_len(ctx);
            }
        }
        if encoding_mask.contains(DataValueFlags::HAS_SERVER_TIMESTAMP) {
            size += self.server_timestamp.as_ref().unwrap().byte_len(ctx);
            if encoding_mask.contains(DataValueFlags::HAS_SERVER_PICOSECONDS) {
                size += self.server_picoseconds.as_ref().unwrap().byte_len(ctx);
            }
        }
        size
    }

    fn encode<S: Write + ?Sized>(&self, stream: &mut S, ctx: &Context<'_>) -> EncodingResult<()> {
        let encoding_mask = self.encoding_mask();
        encoding_mask.bits().encode(stream, ctx)?;

        if encoding_mask.contains(DataValueFlags::HAS_VALUE) {
            self.value.as_ref().unwrap().encode(stream, ctx)?;
        }
        if encoding_mask.contains(DataValueFlags::HAS_STATUS) {
            self.status.as_ref().unwrap().bits().encode(stream, ctx)?;
        }
        if encoding_mask.contains(DataValueFlags::HAS_SOURCE_TIMESTAMP) {
            self.source_timestamp
                .as_ref()
                .unwrap()
                .encode(stream, ctx)?;
            if encoding_mask.contains(DataValueFlags::HAS_SOURCE_PICOSECONDS) {
                self.source_picoseconds
                    .as_ref()
                    .unwrap()
                    .encode(stream, ctx)?;
            }
        }
        if encoding_mask.contains(DataValueFlags::HAS_SERVER_TIMESTAMP) {
            self.server_timestamp
                .as_ref()
                .unwrap()
                .encode(stream, ctx)?;
            if encoding_mask.contains(DataValueFlags::HAS_SERVER_PICOSECONDS) {
                self.server_picoseconds
                    .as_ref()
                    .unwrap()
                    .encode(stream, ctx)?;
            }
        }
        Ok(())
    }
}

impl BinaryDecodable for DataValue {
    fn decode<S: Read + ?Sized>(stream: &mut S, ctx: &Context<'_>) -> EncodingResult<Self> {
        let encoding_mask = DataValueFlags::from_bits_truncate(u8::decode(stream, ctx)?);

        // Value
        let value = if encoding_mask.contains(DataValueFlags::HAS_VALUE) {
            Some(Variant::decode(stream, ctx)?)
        } else {
            None
        };
        // Status
        let status = if encoding_mask.contains(DataValueFlags::HAS_STATUS) {
            let status = StatusCode::from(u32::decode(stream, ctx)?);
            Some(status)
        } else {
            None
        };
        // Source timestamp
        let source_timestamp = if encoding_mask.contains(DataValueFlags::HAS_SOURCE_TIMESTAMP) {
            // The source timestamp should never be adjusted, not even when ignoring clock skew

            let ctx = ctx.with_zero_offset();
            Some(DateTime::decode(stream, &ctx)?)
        } else {
            None
        };
        let source_picoseconds = if encoding_mask.contains(DataValueFlags::HAS_SOURCE_PICOSECONDS) {
            Some(u16::decode(stream, ctx)?)
        } else {
            None
        };
        // Server timestamp
        let server_timestamp = if encoding_mask.contains(DataValueFlags::HAS_SERVER_TIMESTAMP) {
            Some(DateTime::decode(stream, ctx)?)
        } else {
            None
        };
        let server_picoseconds = if encoding_mask.contains(DataValueFlags::HAS_SERVER_PICOSECONDS) {
            Some(u16::decode(stream, ctx)?)
        } else {
            None
        };
        // Pico second values are discarded if associated timestamp is not supplied
        Ok(DataValue {
            value,
            status,
            source_picoseconds: if source_timestamp.is_some() {
                source_picoseconds
            } else {
                None
            },
            source_timestamp,
            server_picoseconds: if server_timestamp.is_some() {
                server_picoseconds
            } else {
                None
            },
            server_timestamp,
        })
    }
}

// It would be nice if everything from here to the ... below could be condensed into a single
// trait impl somehow because it's more or less duplicating all the code in Variant.

impl From<bool> for DataValue {
    fn from(v: bool) -> Self {
        Self::from(Variant::from(v))
    }
}

impl From<u8> for DataValue {
    fn from(v: u8) -> Self {
        Self::from(Variant::from(v))
    }
}

impl From<i8> for DataValue {
    fn from(v: i8) -> Self {
        Self::from(Variant::from(v))
    }
}

impl From<i16> for DataValue {
    fn from(v: i16) -> Self {
        Self::from(Variant::from(v))
    }
}

impl From<u16> for DataValue {
    fn from(v: u16) -> Self {
        Self::from(Variant::from(v))
    }
}

impl From<i32> for DataValue {
    fn from(v: i32) -> Self {
        Self::from(Variant::from(v))
    }
}

impl From<u32> for DataValue {
    fn from(v: u32) -> Self {
        Self::from(Variant::from(v))
    }
}

impl From<i64> for DataValue {
    fn from(v: i64) -> Self {
        Self::from(Variant::from(v))
    }
}

impl From<u64> for DataValue {
    fn from(v: u64) -> Self {
        Self::from(Variant::from(v))
    }
}

impl From<f32> for DataValue {
    fn from(v: f32) -> Self {
        Self::from(Variant::from(v))
    }
}

impl From<f64> for DataValue {
    fn from(v: f64) -> Self {
        Self::from(Variant::from(v))
    }
}

impl<'a> From<&'a str> for DataValue {
    fn from(v: &'a str) -> Self {
        Self::from(Variant::from(v))
    }
}

impl From<String> for DataValue {
    fn from(v: String) -> Self {
        Self::from(Variant::from(v))
    }
}

impl From<UAString> for DataValue {
    fn from(v: UAString) -> Self {
        Self::from(Variant::from(v))
    }
}

impl From<DateTime> for DataValue {
    fn from(v: DateTime) -> Self {
        Self::from(Variant::from(v))
    }
}

impl From<Guid> for DataValue {
    fn from(v: Guid) -> Self {
        Self::from(Variant::from(v))
    }
}

impl From<StatusCode> for DataValue {
    fn from(v: StatusCode) -> Self {
        Self::from(Variant::from(v))
    }
}

impl From<ByteString> for DataValue {
    fn from(v: ByteString) -> Self {
        Self::from(Variant::from(v))
    }
}

impl From<QualifiedName> for DataValue {
    fn from(v: QualifiedName) -> Self {
        Self::from(Variant::from(v))
    }
}

impl From<LocalizedText> for DataValue {
    fn from(v: LocalizedText) -> Self {
        Self::from(Variant::from(v))
    }
}

impl From<NodeId> for DataValue {
    fn from(v: NodeId) -> Self {
        Self::from(Variant::from(v))
    }
}
//... (see above)

impl From<Variant> for DataValue {
    fn from(v: Variant) -> Self {
        DataValue::value_only(v)
    }
}

impl From<(Variant, StatusCode)> for DataValue {
    fn from(v: (Variant, StatusCode)) -> Self {
        DataValue {
            value: Some(v.0),
            status: Some(v.1),
            source_timestamp: None,
            source_picoseconds: None,
            server_timestamp: None,
            server_picoseconds: None,
        }
    }
}

/*
impl<'a> From<(Variant, &'a DateTime)> for DataValue {
    fn from(v: (Variant, &'a DateTime)) -> Self {
        DataValue {
            value: Some(v.0),
            status: Some(StatusCode::Good),
            source_timestamp: Some(v.1.clone()),
            source_picoseconds: Some(0),
            server_timestamp: Some(v.1.clone()),
            server_picoseconds: Some(0),
        }
    }
}

impl<'a> From<(Variant, &'a DateTime, &'a DateTime)> for DataValue {
    fn from(v: (Variant, &'a DateTime, &'a DateTime)) -> Self {
        // First date is source time, second is server time
        DataValue {
            value: Some(v.0),
            status: Some(StatusCode::Good),
            source_timestamp: Some(v.1.clone()),
            source_picoseconds: Some(0),
            server_timestamp: Some(v.2.clone()),
            server_picoseconds: Some(0),
        }
    }
}
*/

impl Default for DataValue {
    fn default() -> Self {
        Self::null()
    }
}

impl DataValue {
    /// Creates a `DataValue` from the supplied value with nothing else.
    pub fn value_only<V>(value: V) -> DataValue
    where
        V: Into<Variant>,
    {
        DataValue {
            value: Some(value.into()),
            status: None,
            source_timestamp: None,
            source_picoseconds: None,
            server_timestamp: None,
            server_picoseconds: None,
        }
    }

    /// Creates a `DataValue` from the supplied value AND a timestamp for now. If you are passing a value to the Attribute::Write service
    /// on a server from a server, you may consider this from the specification:
    ///
    /// _If the SourceTimestamp or the ServerTimestamp is specified, the Server shall use these values.
    /// The Server returns a Bad_WriteNotSupported error if it does not support writing of timestamps_
    ///
    /// In which case, use the `value_only()` constructor, or make explicit which fields you pass.
    pub fn new_now<V>(value: V) -> DataValue
    where
        V: Into<Variant>,
    {
        let now = DateTime::now();
        DataValue {
            value: Some(value.into()),
            status: Some(StatusCode::Good),
            source_timestamp: Some(now),
            source_picoseconds: Some(0),
            server_timestamp: Some(now),
            server_picoseconds: Some(0),
        }
    }

    /// Creates a `DataValue` from the supplied value and timestamp. If you are passing a value to the Attribute::Write service
    /// on a server from a server, you may consider this from the specification:
    ///
    /// _If the SourceTimestamp or the ServerTimestamp is specified, the Server shall use these values.
    /// The Server returns a Bad_WriteNotSupported error if it does not support writing of timestamps_
    ///
    /// In which case, use the `value_only()` constructor, or make explicit which fields you pass.
    pub fn new_at<V>(value: V, time: DateTime) -> DataValue
    where
        V: Into<Variant>,
    {
        DataValue {
            value: Some(value.into()),
            status: Some(StatusCode::Good),
            source_timestamp: Some(time),
            source_picoseconds: Some(0),
            server_timestamp: Some(time),
            server_picoseconds: Some(0),
        }
    }

    /// Creates a `DataValue` from the supplied value AND a timestamp for now. If you are passing a value to the Attribute::Write service
    /// on a server from a server, you may consider this from the specification:
    ///
    /// _If the SourceTimestamp or the ServerTimestamp is specified, the Server shall use these values.
    /// The Server returns a Bad_WriteNotSupported error if it does not support writing of timestamps_
    ///
    /// In which case, use the `value_only()` constructor, or make explicit which fields you pass.
    pub fn new_now_status<V>(value: V, status: StatusCode) -> DataValue
    where
        V: Into<Variant>,
    {
        let now = DateTime::now();
        DataValue {
            value: Some(value.into()),
            status: Some(status),
            source_timestamp: Some(now),
            source_picoseconds: Some(0),
            server_timestamp: Some(now),
            server_picoseconds: Some(0),
        }
    }

    /// Creates a `DataValue` from the supplied value and timestamp. If you are passing a value to the Attribute::Write service
    /// on a server from a server, you may consider this from the specification:
    ///
    /// _If the SourceTimestamp or the ServerTimestamp is specified, the Server shall use these values.
    /// The Server returns a Bad_WriteNotSupported error if it does not support writing of timestamps_
    ///
    /// In which case, use the `value_only()` constructor, or make explicit which fields you pass.
    pub fn new_at_status<V>(value: V, time: DateTime, status: StatusCode) -> DataValue
    where
        V: Into<Variant>,
    {
        DataValue {
            value: Some(value.into()),
            status: Some(status),
            source_timestamp: Some(time),
            source_picoseconds: Some(0),
            server_timestamp: Some(time),
            server_picoseconds: Some(0),
        }
    }

    /// Creates an empty DataValue
    pub fn null() -> DataValue {
        DataValue {
            value: None,
            status: None,
            source_timestamp: None,
            source_picoseconds: None,
            server_timestamp: None,
            server_picoseconds: None,
        }
    }

    /// Sets the value of the data value, updating the timestamps at the same point
    pub fn set_value<V>(
        &mut self,
        value: V,
        source_timestamp: &DateTime,
        server_timestamp: &DateTime,
    ) where
        V: Into<Variant>,
    {
        self.value = Some(value.into());
        self.source_timestamp = Some(*source_timestamp);
        self.source_picoseconds = Some(0);
        self.server_timestamp = Some(*server_timestamp);
        self.server_picoseconds = Some(0);
    }

    /// Sets the timestamps of the data value based on supplied timestamps to return
    pub fn set_timestamps(
        &mut self,
        timestamps_to_return: TimestampsToReturn,
        source_timestamp: DateTime,
        server_timestamp: DateTime,
    ) {
        match timestamps_to_return {
            TimestampsToReturn::Source => {
                self.source_timestamp = Some(source_timestamp);
                self.source_picoseconds = Some(0);
                self.server_timestamp = None;
                self.server_picoseconds = None;
            }
            TimestampsToReturn::Server => {
                self.source_timestamp = None;
                self.source_picoseconds = None;
                self.server_timestamp = Some(server_timestamp);
                self.server_picoseconds = Some(0);
            }
            TimestampsToReturn::Both => {
                self.source_timestamp = Some(source_timestamp);
                self.source_picoseconds = Some(0);
                self.server_timestamp = Some(server_timestamp);
                self.server_picoseconds = Some(0);
            }
            TimestampsToReturn::Neither => {
                self.source_timestamp = None;
                self.source_picoseconds = None;
                self.server_timestamp = None;
                self.server_picoseconds = None;
            }
            _ => {}
        }
    }

    /// Returns the status code or Good if there is no code on the value
    pub fn status(&self) -> StatusCode {
        self.status.map_or(StatusCode::Good, |s| s)
    }

    /// Test if the value held by this data value is known to be good
    /// Anything other than Good is assumed to be invalid.
    pub fn is_valid(&self) -> bool {
        self.status().is_good()
    }

    fn encoding_mask(&self) -> DataValueFlags {
        let mut encoding_mask = DataValueFlags::empty();
        if self.value.is_some() {
            encoding_mask |= DataValueFlags::HAS_VALUE;
        }
        if self.status.is_some() {
            encoding_mask |= DataValueFlags::HAS_STATUS;
        }
        if self.source_timestamp.is_some() {
            encoding_mask |= DataValueFlags::HAS_SOURCE_TIMESTAMP;
            if self.source_picoseconds.is_some() {
                encoding_mask |= DataValueFlags::HAS_SOURCE_PICOSECONDS;
            }
        }
        if self.server_timestamp.is_some() {
            encoding_mask |= DataValueFlags::HAS_SERVER_TIMESTAMP;
            if self.server_picoseconds.is_some() {
                encoding_mask |= DataValueFlags::HAS_SERVER_PICOSECONDS;
            }
        }
        encoding_mask
    }
}
