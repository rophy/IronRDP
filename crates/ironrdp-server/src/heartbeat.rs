//! Server-side Heartbeat PDU emission (MS-RDPBCGR 2.2.16).

use std::sync::{Arc, Mutex};

use tokio::sync::Notify;

/// Configuration for periodic Server Heartbeat PDUs (MS-RDPBCGR 2.2.16.1).
///
/// Heartbeats let the client monitor connection health in real time: when it
/// stops receiving them (and everything else) for long enough, it can warn
/// the user or start a reconnect attempt. They ride the MCS message channel
/// and are only sent while the connection is otherwise idle; ordinary
/// traffic doubles as the liveness signal.
///
/// The specification mandates no particular values, so the defaults are this
/// library's own choice: a 5-second period keeps idle-link detection
/// responsive at negligible traffic cost (8 bytes of payload per interval),
/// `warning_count` = 3 puts a client-side warning at roughly 15 seconds of
/// silence, and `reconnect_count` = 5 a reconnect attempt at roughly
/// 40 seconds. The client MAY ignore both counts (2.2.16.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeartbeatConfig {
    /// Seconds between heartbeats while the connection is otherwise idle.
    ///
    /// `0` is treated as `1`.
    pub period_secs: u8,
    /// Missed heartbeats that SHOULD trigger a client-side warning.
    pub warning_count: u8,
    /// Missed heartbeats after the warning that SHOULD trigger a client-side
    /// reconnect attempt.
    pub reconnect_count: u8,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            period_secs: 5,
            warning_count: 3,
            reconnect_count: 5,
        }
    }
}

/// Cloneable handle that sends one Server Heartbeat PDU on demand, while
/// [`RdpServer::run`](crate::RdpServer::run) owns the server. Get it from
/// [`RdpServer::heartbeat_handle`](crate::RdpServer::heartbeat_handle).
///
/// Use it to put a small packet on the wire when the application wants one,
/// besides or without [`RdpServer::enable_heartbeat`](crate::RdpServer::enable_heartbeat).
/// The client sends no reply.
#[derive(Clone, Default)]
pub struct HeartbeatHandle {
    inner: Arc<HeartbeatRequest>,
}

#[derive(Default)]
struct HeartbeatRequest {
    pending: Mutex<Option<HeartbeatConfig>>,
    notify: Notify,
}

impl HeartbeatHandle {
    /// Send one Heartbeat PDU with `config` to the current client, if it
    /// advertised heartbeat support and has a message channel
    /// ([`ConnectionInfo::client_supports_heartbeat`](crate::ConnectionInfo::client_supports_heartbeat));
    /// otherwise nothing is sent.
    ///
    /// Requests that come before the server took the last one are merged into
    /// one PDU with the newest `config`. A request made while no client is
    /// connected is dropped when the next client loop starts.
    ///
    /// # Panics
    ///
    /// Panics if the request lock is poisoned (a thread panicked while
    /// holding it).
    pub fn send(&self, config: HeartbeatConfig) {
        *self.inner.pending.lock().expect("heartbeat request lock poisoned") = Some(config);
        self.inner.notify.notify_one();
    }

    /// Takes the pending request, if any.
    pub(crate) fn take(&self) -> Option<HeartbeatConfig> {
        self.inner
            .pending
            .lock()
            .expect("heartbeat request lock poisoned")
            .take()
    }

    /// Waits for the next request. Cancel safe: a request is taken only when
    /// this returns.
    pub(crate) async fn requested(&self) -> HeartbeatConfig {
        loop {
            if let Some(config) = self.take() {
                return config;
            }
            self.inner.notify.notified().await;
        }
    }
}

impl core::fmt::Debug for HeartbeatHandle {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("HeartbeatHandle").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: HeartbeatConfig = HeartbeatConfig {
        period_secs: 255,
        warning_count: 255,
        reconnect_count: 255,
    };

    #[tokio::test]
    async fn a_request_is_taken_once() {
        let handle = HeartbeatHandle::default();
        handle.clone().send(HeartbeatConfig::default());
        handle.send(CONFIG);
        assert_eq!(handle.requested().await, CONFIG, "the newest config wins");
        let next = tokio::time::timeout(core::time::Duration::from_millis(20), handle.requested()).await;
        assert!(next.is_err(), "two requests before a take make one PDU");
    }

    /// The client loop drops a stale request with `take`. That the loop does
    /// so is tested end to end (testsuite-extra
    /// `a_heartbeat_request_from_before_the_client_loop_is_not_sent`).
    #[tokio::test]
    async fn a_taken_request_is_not_returned_again() {
        let handle = HeartbeatHandle::default();
        handle.send(CONFIG);
        assert_eq!(handle.take(), Some(CONFIG));
        let next = tokio::time::timeout(core::time::Duration::from_millis(20), handle.requested()).await;
        assert!(next.is_err());
    }
}
