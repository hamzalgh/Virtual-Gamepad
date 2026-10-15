//! Protocol and UDP foundation for the Windows agent.
pub mod application;
pub mod config;
#[cfg(windows)]
pub mod desktop;
pub mod desktop_settings;
pub mod discovery;
pub mod input;
pub mod network;
pub mod paired_session;
pub mod pairing;
pub mod pairing_store;
pub mod protocol;
pub mod session;
pub mod telemetry;
pub mod terminal;
pub mod virtual_gamepad;
#[cfg(windows)]
mod windows_network;
#[cfg(windows)]
mod windows_pairing_window;
#[cfg(windows)]
mod windows_storage;
#[cfg(windows)]
pub mod windows_tray;
