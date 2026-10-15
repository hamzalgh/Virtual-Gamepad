//! Native Private-profile inventory. All COM use stays on this calling thread.
use std::{collections::HashSet, net::Ipv4Addr};
use windows::Win32::{
    Networking::NetworkListManager::{
        INetworkListManager, NLM_NETWORK_CATEGORY_PRIVATE, NetworkListManager,
    },
    System::Com::{
        CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
    },
};

struct Apartment;
impl Drop for Apartment {
    fn drop(&mut self) {
        // SAFETY: created only after successful CoInitializeEx on this same thread.
        unsafe {
            CoUninitialize();
        }
    }
}

pub fn private_ipv4() -> Result<Vec<Ipv4Addr>, String> {
    // SAFETY: This function is synchronous on its own discovery thread. Owned COM
    // wrappers drop before the apartment guard. No COM pointer crosses a thread.
    let allowed = unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED)
            .ok()
            .map_err(|e| e.to_string())?;
        let _apartment = Apartment;
        let manager: INetworkListManager =
            CoCreateInstance(&NetworkListManager, None, CLSCTX_ALL).map_err(|e| e.to_string())?;
        let connections = manager.GetNetworkConnections().map_err(|e| e.to_string())?;
        let mut private = HashSet::new();
        let mut excluded = HashSet::new();
        let mut exhausted = false;
        for _ in 0..256 {
            let mut item = [None];
            let mut count = 0;
            connections
                .Next(&mut item, Some(&mut count))
                .map_err(|e| e.to_string())?;
            if count == 0 {
                exhausted = true;
                break;
            }
            let connection = item[0].take().ok_or("Missing network connection")?;
            let id = format!(
                "{:?}",
                connection.GetAdapterId().map_err(|e| e.to_string())?
            )
            .to_ascii_lowercase();
            if connection
                .IsConnected()
                .map_err(|e| e.to_string())?
                .as_bool()
                && connection
                    .GetNetwork()
                    .and_then(|n| n.GetCategory())
                    .map_err(|e| e.to_string())?
                    == NLM_NETWORK_CATEGORY_PRIVATE
            {
                private.insert(id);
            } else {
                excluded.insert(id);
            }
        }
        if !exhausted {
            return Err("Network inventory exceeds safety bound".into());
        }
        private.retain(|id| !excluded.contains(id));
        private
    };
    let interfaces = if_addrs::get_if_addrs().map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    for interface in interfaces.into_iter().take(256) {
        let id = interface
            .adapter_name
            .trim_matches(['{', '}'])
            .to_ascii_lowercase();
        if !allowed.contains(&id) {
            continue;
        }
        if let std::net::IpAddr::V4(ip) = interface.ip()
            && (ip.is_private() || ip.is_link_local())
            && !ip.is_loopback()
        {
            result.push(ip);
        }
    }
    result.sort();
    result.dedup();
    Ok(result)
}
