//! DNS-SD advertisement is isolated from UDP input and fails closed on profile errors.
use mdns_sd::{IfKind, ServiceDaemon, ServiceInfo};
use std::{
    net::{Ipv4Addr, SocketAddrV4},
    sync::mpsc,
    time::Duration,
};
pub const SERVICE_TYPE: &str = "_virtual-gamepad._udp.local.";

pub fn service_name(name: &str, server: u64) -> String {
    let name: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == ' ')
        .take(40)
        .collect();
    format!(
        "{} {:06x}",
        if name.trim().is_empty() {
            "Virtual Gamepad"
        } else {
            name.trim()
        },
        server & 0x00ff_ffff
    )
}
pub fn metadata() -> [(&'static str, &'static str); 4] {
    [
        ("v", "1"),
        ("agent", env!("CARGO_PKG_VERSION")),
        ("capacity", "1"),
        ("pairing", "required"),
    ]
}
pub fn allowed_addresses(mut addresses: Vec<Ipv4Addr>, bound: SocketAddrV4) -> Vec<Ipv4Addr> {
    addresses.retain(|ip| {
        (ip.is_private() || ip.is_link_local()) && (bound.ip().is_unspecified() || bound.ip() == ip)
    });
    addresses.sort();
    addresses.dedup();
    addresses.truncate(32);
    addresses
}
struct Advertisement {
    daemon: ServiceDaemon,
    fullname: String,
}
impl Advertisement {
    fn start(
        name: &str,
        server: u64,
        bound: SocketAddrV4,
        addresses: &[Ipv4Addr],
    ) -> Result<Self, String> {
        let daemon = ServiceDaemon::new().map_err(|e| e.to_string())?;
        let result = (|| {
            daemon
                .disable_interface(IfKind::All)
                .map_err(|e| e.to_string())?;
            // A fresh daemon per inventory change avoids an unbounded interface-rule list.
            daemon.set_ip_check_interval(0).map_err(|e| e.to_string())?;
            let interfaces: Vec<IfKind> = addresses
                .iter()
                .map(|ip| IfKind::Addr((*ip).into()))
                .collect();
            daemon
                .enable_interface(interfaces.clone())
                .map_err(|e| e.to_string())?;
            let ips = addresses
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",");
            let hostname = format!("vg-{:06x}.local.", server & 0x00ff_ffff);
            let mut info = ServiceInfo::new(
                SERVICE_TYPE,
                &service_name(name, server),
                &hostname,
                ips.as_str(),
                bound.port(),
                &metadata()[..],
            )
            .map_err(|e| e.to_string())?;
            info.set_interfaces(interfaces);
            let fullname = info.get_fullname().to_owned();
            daemon.register(info).map_err(|e| e.to_string())?;
            Ok(Self {
                daemon: daemon.clone(),
                fullname,
            })
        })();
        if result.is_err() {
            let _ = daemon.shutdown();
        }
        result
    }
    fn stop(self, withdraw: bool) {
        // On profile loss disable transmission before unregistering. For ordinary
        // shutdown, goodbye records let other devices remove us promptly.
        if withdraw {
            let _ = self.daemon.disable_interface(IfKind::All);
        }
        if let Ok(done) = self.daemon.unregister(&self.fullname) {
            let _ = done.recv_timeout(Duration::from_millis(200));
        }
        if let Ok(done) = self.daemon.shutdown() {
            let _ = done.recv_timeout(Duration::from_secs(1));
        }
    }
}
pub struct Discovery {
    stop: mpsc::Sender<()>,
    task: Option<std::thread::JoinHandle<()>>,
}
impl Discovery {
    pub fn start(name: String, server: u64, bound: SocketAddrV4) -> std::io::Result<Self> {
        let (stop, rx) = mpsc::channel();
        let task=std::thread::Builder::new().name("private-discovery".into()).spawn(move || {
            let mut previous=Vec::new();let mut current:Option<Advertisement>=None;let mut warned=false;
            let mut retry_at=std::time::Instant::now();
            loop {
                #[cfg(windows)] let inventory=crate::windows_network::private_ipv4();
                #[cfg(not(windows))] let inventory:Result<Vec<Ipv4Addr>,String>=Err("Windows profile detection is required".into());
                let addresses=match inventory {Ok(ips)=>allowed_addresses(ips,bound),Err(_)=>{if !warned{tracing::warn!("Private network inspection unavailable; discovery withdrawn, use manual IP");warned=true;}Vec::new()}};
                if addresses!=previous || (current.is_none() && !addresses.is_empty() && std::time::Instant::now()>=retry_at) {
                    if let Some(ad)=current.take(){ad.stop(true);}
                    previous=addresses.clone();
                    if !addresses.is_empty() {
                        match Advertisement::start(&name,server,bound,&addresses) {
                            Ok(ad)=>{current=Some(ad);warned=false;tracing::info!("Private LAN discovery registered; multicast delivery still depends on the network and firewall");}
                            Err(_)=>{if !warned {tracing::warn!("Discovery unavailable; manual IP remains available");warned=true;}}
                        }
                        retry_at=std::time::Instant::now()+Duration::from_secs(5);
                    }
                }
                match rx.recv_timeout(Duration::from_millis(500)) {Err(mpsc::RecvTimeoutError::Timeout)=>{},_=>break}
            }
            if let Some(ad)=current{ad.stop(false);}
        })?;
        Ok(Self {
            stop,
            task: Some(task),
        })
    }
}
impl Drop for Discovery {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        if let Some(task) = self.task.take() {
            let _ = task.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn metadata_has_only_public_hints_and_short_suffix() {
        assert_eq!(
            metadata().map(|(k, _)| k),
            ["v", "agent", "capacity", "pairing"]
        );
        let name = service_name("Bad\nPC._secret", 0x123456abcdef);
        assert_eq!(name, "BadPCsecret abcdef");
        assert!(!name.contains("123456"));
        assert!(service_name(&"x".repeat(200), 1).len() <= 63);
    }
    #[test]
    fn advertisement_filters_public_loopback_and_nonbound_addresses() {
        let ips = vec![
            Ipv4Addr::new(192, 168, 1, 2),
            Ipv4Addr::new(10, 0, 0, 2),
            Ipv4Addr::LOCALHOST,
            Ipv4Addr::new(8, 8, 8, 8),
        ];
        assert_eq!(
            allowed_addresses(ips.clone(), "0.0.0.0:8765".parse().unwrap()).len(),
            2
        );
        assert_eq!(
            allowed_addresses(ips, "192.168.1.2:8765".parse().unwrap()),
            vec![Ipv4Addr::new(192, 168, 1, 2)]
        );
        assert!(
            allowed_addresses(vec![Ipv4Addr::LOCALHOST], "127.0.0.1:8765".parse().unwrap())
                .is_empty()
        );
    }
}
