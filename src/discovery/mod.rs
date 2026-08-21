//! mDNS advertise and browse for clipboard-share peers.

use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub const SERVICE_TYPE: &str = "_clipboard-share._tcp.local.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredPeer {
    pub instance: String,
    pub host: String,
    pub port: u16,
    pub display_name: String,
}

#[derive(Debug, Clone)]
pub enum DiscoveryError {
    Failed(String),
}

impl std::fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiscoveryError::Failed(s) => write!(f, "{}", s),
        }
    }
}

pub struct DiscoveryService {
    daemon: ServiceDaemon,
    peers: Arc<Mutex<HashMap<String, DiscoveredPeer>>>,
    last_error: Arc<Mutex<Option<String>>>,
}

impl DiscoveryService {
    pub fn new() -> Result<Self, DiscoveryError> {
        let daemon = ServiceDaemon::new().map_err(|e| DiscoveryError::Failed(e.to_string()))?;
        Ok(Self {
            daemon,
            peers: Arc::new(Mutex::new(HashMap::new())),
            last_error: Arc::new(Mutex::new(None)),
        })
    }

    pub fn last_error(&self) -> Option<String> {
        self.last_error.lock().ok().and_then(|g| g.clone())
    }

    pub fn advertise(
        &self,
        instance: &str,
        host: &str,
        port: u16,
        display_name: &str,
    ) -> Result<(), DiscoveryError> {
        let mut properties = HashMap::new();
        properties.insert("dn".to_string(), display_name.to_string());
        properties.insert("v".to_string(), "1".to_string());
        let service = ServiceInfo::new(
            SERVICE_TYPE,
            instance,
            host,
            host,
            port,
            Some(properties),
        )
        .map_err(|e| DiscoveryError::Failed(e.to_string()))?;
        self.daemon
            .register(service)
            .map_err(|e| DiscoveryError::Failed(e.to_string()))?;
        Ok(())
    }

    pub fn start_browse(&self) -> Result<(), DiscoveryError> {
        let receiver = self
            .daemon
            .browse(SERVICE_TYPE)
            .map_err(|e| DiscoveryError::Failed(e.to_string()))?;
        let peers = self.peers.clone();
        let last_error = self.last_error.clone();
        std::thread::spawn(move || {
            while let Ok(event) = receiver.recv() {
                match event {
                    ServiceEvent::ServiceResolved(info) => {
                        let display_name = info
                            .get_properties()
                            .get("dn")
                            .map(|v| v.to_string())
                            .unwrap_or_else(|| info.get_fullname().to_string());
                        let host = info
                            .get_addresses()
                            .iter()
                            .next()
                            .map(|a| a.to_string())
                            .unwrap_or_else(|| info.get_hostname().to_string());
                        let peer = DiscoveredPeer {
                            instance: info.get_fullname().to_string(),
                            host,
                            port: info.get_port(),
                            display_name,
                        };
                        if let Ok(mut g) = peers.lock() {
                            g.insert(peer.instance.clone(), peer);
                        }
                    }
                    ServiceEvent::ServiceRemoved(_, fullname) => {
                        if let Ok(mut g) = peers.lock() {
                            g.remove(&fullname);
                        }
                    }
                    _ => {}
                }
            }
            if let Ok(mut e) = last_error.lock() {
                *e = Some("mDNS browse ended".into());
            }
        });
        Ok(())
    }

    pub fn peers(&self) -> Vec<DiscoveredPeer> {
        self.peers
            .lock()
            .map(|g| g.values().cloned().collect())
            .unwrap_or_default()
    }
}

/// Pure helper: manual IP remains available even when peer list is empty.
pub fn can_manual_connect(discovered_count: usize) -> bool {
    let _ = discovered_count;
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_type_is_clipboard_share() {
        assert!(SERVICE_TYPE.contains("clipboard-share"));
        assert!(SERVICE_TYPE.ends_with("._tcp.local."));
    }

    #[test]
    fn manual_connect_always_allowed() {
        assert!(can_manual_connect(0));
        assert!(can_manual_connect(3));
    }

    #[test]
    fn daemon_starts_or_reports_error() {
        // On restricted environments this may fail; error must be observable.
        match DiscoveryService::new() {
            Ok(svc) => {
                assert!(svc.last_error().is_none());
                let _ = svc.start_browse();
            }
            Err(e) => {
                let msg = e.to_string();
                assert!(!msg.is_empty());
            }
        }
    }
}
