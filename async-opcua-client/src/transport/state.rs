use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use tokio::sync::mpsc::error::SendTimeoutError;
use tracing::{debug, trace};

use crate::{session::process_unexpected_response, transport::OutgoingMessage};
use arc_swap::ArcSwap;
use opcua_core::{
    comms::secure_channel::SecureChannel, handle::AtomicHandle, sync::RwLock, trace_write_lock,
    RequestMessage, ResponseMessage,
};
use opcua_crypto::SecurityPolicy;
use opcua_types::{
    DateTime, DiagnosticBits, IntegerId, MessageSecurityMode, NodeId, OpenSecureChannelRequest,
    RequestHeader, SecurityTokenRequestType, StatusCode,
};

pub(crate) type RequestSend = tokio::sync::mpsc::Sender<OutgoingMessage>;

pub(super) struct SecureChannelState {
    /// Time offset between the client and the server.
    client_offset: ArcSwap<chrono::Duration>,
    /// Ignore clock skew between the client and the server.
    ignore_clock_skew: bool,
    /// Secure channel information
    secure_channel: Arc<RwLock<SecureChannel>>,
    /// The session authentication token, used for session activation
    authentication_token: Arc<ArcSwap<NodeId>>,
    /// The next handle to assign to a request
    request_handle: AtomicHandle,
}

pub(super) struct Request {
    payload: RequestMessage,
    sender: RequestSend,
    timeout: std::time::Duration,
}

impl Request {
    pub(super) fn new(
        payload: impl Into<RequestMessage>,
        sender: RequestSend,
        timeout: Duration,
    ) -> Self {
        Self {
            payload: payload.into(),
            sender,
            timeout,
        }
    }

    pub(super) async fn send_no_response(self) -> Result<(), StatusCode> {
        let message = OutgoingMessage {
            request: self.payload,
            callback: None,
            deadline: Instant::now() + self.timeout,
        };

        match self.sender.send_timeout(message, self.timeout).await {
            Ok(()) => Ok(()),
            Err(SendTimeoutError::Closed(_)) => Err(StatusCode::BadConnectionClosed),
            Err(SendTimeoutError::Timeout(_)) => Err(StatusCode::BadTimeout),
        }
    }

    pub(super) async fn send(self) -> Result<ResponseMessage, StatusCode> {
        let (cb_send, cb_recv) = tokio::sync::oneshot::channel();

        let message = OutgoingMessage {
            request: self.payload,
            callback: Some(cb_send),
            deadline: Instant::now() + self.timeout,
        };

        match self.sender.send_timeout(message, self.timeout).await {
            Ok(()) => (),
            Err(SendTimeoutError::Closed(_)) => return Err(StatusCode::BadConnectionClosed),
            Err(SendTimeoutError::Timeout(_)) => return Err(StatusCode::BadTimeout),
        }

        match cb_recv.await {
            Ok(r) => r,
            // Should not really happen, would mean something panicked.
            Err(_) => Err(StatusCode::BadConnectionClosed),
        }
    }
}

impl SecureChannelState {
    const FIRST_REQUEST_HANDLE: u32 = 1;

    pub(super) fn new(
        ignore_clock_skew: bool,
        secure_channel: Arc<RwLock<SecureChannel>>,
        authentication_token: Arc<ArcSwap<NodeId>>,
    ) -> Self {
        SecureChannelState {
            client_offset: ArcSwap::new(Arc::new(chrono::Duration::zero())),
            ignore_clock_skew,
            secure_channel,
            authentication_token,
            request_handle: AtomicHandle::new(Self::FIRST_REQUEST_HANDLE),
        }
    }

    pub(super) fn begin_issue_or_renew_secure_channel(
        &self,
        request_type: SecurityTokenRequestType,
        requested_lifetime: u32,
        timeout: Duration,
        sender: RequestSend,
    ) -> Request {
        trace!("issue_or_renew_secure_channel({:?})", request_type);

        let (security_mode, security_policy, client_nonce) = {
            let mut secure_channel = trace_write_lock!(self.secure_channel);
            let client_nonce = secure_channel.security_policy().random_nonce();
            secure_channel.set_local_nonce(client_nonce.as_ref());
            (
                secure_channel.security_mode(),
                secure_channel.security_policy(),
                client_nonce,
            )
        };

        debug!("Making secure channel request");
        debug!("security_mode = {:?}", security_mode);
        debug!("security_policy = {:?}", security_policy);

        let request = OpenSecureChannelRequest {
            request_header: self.make_request_header(timeout),
            client_protocol_version: 0,
            request_type,
            security_mode,
            client_nonce,
            requested_lifetime,
        };

        Request::new(request, sender, timeout)
    }

    pub(super) fn set_client_offset(&self, offset: chrono::Duration) {
        // This is not strictly speaking thread safe, but it doesn't really matter in this case,
        // the assumption is that this is only called from a single thread at once.
        self.client_offset
            .store(Arc::new(**self.client_offset.load() + offset));
        debug!("Client offset set to {}", self.client_offset);
    }

    pub(super) fn end_issue_or_renew_secure_channel(
        &self,
        response: ResponseMessage,
    ) -> Result<(), StatusCode> {
        if let ResponseMessage::OpenSecureChannel(response) = response {
            // Extract the security token from the response.
            let mut security_token = response.security_token.clone();

            // When ignoring clock skew, we calculate the time offset between the client and the
            // server and use that offset to compensate for the difference in time when setting
            // the timestamps in the request headers and when decoding timestamps in messages
            // received from the server.
            if self.ignore_clock_skew && !response.response_header.timestamp.is_null() {
                let offset = response.response_header.timestamp - DateTime::now();
                // Make sure to apply the offset to the security token in the current response.
                security_token.created_at = security_token.created_at - offset;
                // Update the client offset by adding the new offset. When the secure channel is
                // renewed its already using the client offset calculated when issuing the secure
                // channel and only needs to be updated to accommodate any additional clock skew.
                self.set_client_offset(offset);
            }

            debug!("Setting transport's security token");
            {
                let mut secure_channel = trace_write_lock!(self.secure_channel);
                secure_channel.set_client_offset(**self.client_offset.load());
                secure_channel.set_security_token(security_token);

                if secure_channel.security_policy() != SecurityPolicy::None
                    && (secure_channel.security_mode() == MessageSecurityMode::Sign
                        || secure_channel.security_mode() == MessageSecurityMode::SignAndEncrypt)
                {
                    secure_channel.set_remote_nonce_from_byte_string(&response.server_nonce)?;
                    secure_channel.derive_keys();
                }
            }
            Ok(())
        } else {
            Err(process_unexpected_response(response))
        }
    }

    /// Construct a request header for the session. All requests after create session are expected
    /// to supply an authentication token.
    pub(super) fn make_request_header(&self, timeout: Duration) -> RequestHeader {
        RequestHeader {
            authentication_token: self.authentication_token.load().as_ref().clone(),
            timestamp: DateTime::now_with_offset(**self.client_offset.load()),
            request_handle: self.request_handle.next(),
            return_diagnostics: DiagnosticBits::empty(),
            timeout_hint: timeout.as_millis().min(u32::MAX as u128) as u32,
            ..Default::default()
        }
    }

    pub(super) fn request_handle(&self) -> IntegerId {
        self.request_handle.next()
    }

    pub(super) fn set_auth_token(&self, token: NodeId) {
        self.authentication_token.store(Arc::new(token));
    }
}
