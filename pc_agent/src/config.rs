//! IPv4 listener configuration. No hostname resolution or public address defaults.
use std::{
    fmt,
    net::{Ipv4Addr, SocketAddrV4},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    bind: SocketAddrV4,
    pub debug: bool,
    pub manual_trust_first_client: bool,
    pub input_timeout: InputTimeout,
}
/// Validated once, shared by admission, output, hello and status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputTimeout(u16);
impl InputTimeout {
    pub fn new(ms: u16) -> Result<Self, ConfigError> {
        if (100..=1000).contains(&ms) {
            Ok(Self(ms))
        } else {
            Err(ConfigError)
        }
    }
    pub fn millis(self) -> u16 {
        self.0
    }
    pub fn duration(self) -> std::time::Duration {
        std::time::Duration::from_millis(u64::from(self.0))
    }
}
impl Default for InputTimeout {
    fn default() -> Self {
        Self(250)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigError;
impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("usage: virtual-gamepad-agent [--bind PRIVATE_OR_LOOPBACK_IPV4:PORT] [--input-timeout-ms 100..1000] [--debug] [--manual-trust-first-client (debug builds only, unsafe)]; port 1..65535, default 0.0.0.0:8765, timeout 250 ms")
    }
}
impl std::error::Error for ConfigError {}
impl Default for Config {
    fn default() -> Self {
        Self {
            bind: SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 8765),
            debug: false,
            manual_trust_first_client: false,
            input_timeout: InputTimeout::default(),
        }
    }
}
impl Config {
    pub fn from_args(args: impl IntoIterator<Item = String>) -> Result<Self, ConfigError> {
        let mut config = Self::default();
        let mut args = args.into_iter();
        let mut seen_bind = false;
        let mut seen_timeout = false;
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--input-timeout-ms" if !seen_timeout => {
                    config.input_timeout = InputTimeout::new(
                        args.next()
                            .ok_or(ConfigError)?
                            .parse()
                            .map_err(|_| ConfigError)?,
                    )?;
                    seen_timeout = true;
                }
                "--bind" if !seen_bind => {
                    let addr: SocketAddrV4 = args
                        .next()
                        .ok_or(ConfigError)?
                        .parse()
                        .map_err(|_| ConfigError)?;
                    let ip = addr.ip();
                    if addr.port() == 0
                        || !(ip.is_unspecified()
                            || ip.is_loopback()
                            || ip.is_private()
                            || ip.is_link_local())
                    {
                        return Err(ConfigError);
                    }
                    config.bind = addr;
                    seen_bind = true;
                }
                "--debug" if !config.debug => config.debug = true,
                "--manual-trust-first-client"
                    if cfg!(debug_assertions) && !config.manual_trust_first_client =>
                {
                    config.manual_trust_first_client = true
                }
                _ => return Err(ConfigError),
            }
        }
        Ok(config)
    }
    pub fn bind_addr(&self) -> SocketAddrV4 {
        self.bind
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parse(s: &str) -> Result<Config, ConfigError> {
        Config::from_args(s.split_whitespace().map(String::from))
    }
    #[test]
    fn config_defaults_and_overrides() {
        assert_eq!(parse("").unwrap().bind_addr().to_string(), "0.0.0.0:8765");
        let c = parse("--bind 127.0.0.1:9000 --debug").unwrap();
        assert!(c.debug);
        assert_eq!(c.bind_addr().port(), 9000);
        assert!(parse("--bind 192.168.1.2:65535").is_ok());
    }
    #[test]
    fn config_rejects_bad_or_public_endpoints() {
        for args in [
            "--bind",
            "--bind localhost:8765",
            "--bind [::1]:8765",
            "--bind 127.0.0.1:0",
            "--bind 127.0.0.1:65536",
            "--bind 8.8.8.8:8765",
            "--bind 224.0.0.1:8765",
            "--bind 255.255.255.255:8765",
            "--wat",
            "--debug --debug",
            "--bind 127.0.0.1:1 --bind 127.0.0.1:2",
        ] {
            assert!(parse(args).is_err(), "{args}");
        }
    }

    #[test]
    fn timeout_is_bounded_and_default_is_250_ms() {
        assert_eq!(parse("").unwrap().input_timeout.millis(), 250);
        for ms in [100, 250, 1000] {
            assert_eq!(
                parse(&format!("--input-timeout-ms {ms}"))
                    .unwrap()
                    .input_timeout
                    .millis(),
                ms
            );
        }
        for args in [
            "--input-timeout-ms",
            "--input-timeout-ms 99",
            "--input-timeout-ms 1001",
            "--input-timeout-ms -1",
            "--input-timeout-ms 65536",
            "--input-timeout-ms x",
            "--input-timeout-ms 100 --input-timeout-ms 200",
        ] {
            assert!(parse(args).is_err(), "{args}");
        }
    }
}
