//! Small versioned desktop settings, independent of pairing identity storage.
use std::{
    fs,
    io::{self, Read, Write},
    path::Path,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DesktopSettings {
    pub port: u16,
    pub timeout_ms: u16,
    pub discovery: bool,
    pub debug: bool,
}
impl Default for DesktopSettings {
    fn default() -> Self {
        Self {
            port: 8765,
            timeout_ms: 250,
            discovery: true,
            debug: false,
        }
    }
}
impl DesktopSettings {
    pub fn parse(text: &str) -> io::Result<Self> {
        let mut result = Self::default();
        let mut seen = std::collections::HashSet::new();
        for line in text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
        {
            let (key, value) = line.split_once('=').ok_or_else(invalid)?;
            let (key, value) = (key.trim(), value.trim());
            if !seen.insert(key) {
                return Err(invalid());
            }
            match key {
                "version" if value == "1" => {}
                "port" => {
                    result.port = value.parse().map_err(|_| invalid())?;
                    if result.port == 0 {
                        return Err(invalid());
                    }
                }
                "timeout_ms" => {
                    result.timeout_ms = value.parse().map_err(|_| invalid())?;
                    crate::config::InputTimeout::new(result.timeout_ms).map_err(|_| invalid())?;
                }
                "discovery" => result.discovery = value.parse().map_err(|_| invalid())?,
                "log_level" => {
                    result.debug = match value {
                        "info" => false,
                        "debug" => true,
                        _ => return Err(invalid()),
                    }
                }
                _ => return Err(invalid()),
            }
        }
        if seen.len() != 5 || !seen.contains("version") {
            return Err(invalid());
        }
        Ok(result)
    }
    pub fn text(self) -> String {
        format!(
            "# Stop listening before reloading changes. State rate is fixed at 60 Hz.\nversion=1\nport={}\ntimeout_ms={}\ndiscovery={}\nlog_level={}\n",
            self.port,
            self.timeout_ms,
            self.discovery,
            if self.debug { "debug" } else { "info" }
        )
    }
    pub fn load(root: &Path) -> io::Result<Self> {
        let path = root.join("settings.ini");
        crate::pairing_store::reject_link(&path)?;
        let read = (|| {
            let mut bytes = Vec::new();
            fs::File::open(&path)?.take(4097).read_to_end(&mut bytes)?;
            Ok::<_, io::Error>(bytes)
        })();
        match read {
            Ok(bytes) if bytes.len() <= 4096 => {
                Self::parse(std::str::from_utf8(&bytes).map_err(|_| invalid())?)
            }
            Ok(_) => Err(invalid()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                let value = Self::default();
                value.save(root)?;
                Ok(value)
            }
            Err(e) => Err(e),
        }
    }
    pub fn save(self, root: &Path) -> io::Result<()> {
        Self::parse(&self.text())?;
        let path = root.join("settings.ini");
        crate::pairing_store::reject_link(&path)?;
        let temporary = root.join(format!(
            "settings-{}.tmp",
            crate::session::random_id().ok_or_else(invalid)?
        ));
        let result = (|| {
            let mut file = fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temporary)?;
            file.write_all(self.text().as_bytes())?;
            file.sync_all()?;
            drop(file);
            fs::rename(&temporary, &path)
        })();
        if result.is_err() {
            let _ = fs::remove_file(temporary);
        }
        result
    }
    pub fn config(self) -> crate::config::Config {
        let mut args = vec![
            "--bind".into(),
            format!("0.0.0.0:{}", self.port),
            "--input-timeout-ms".into(),
            self.timeout_ms.to_string(),
        ];
        if self.debug {
            args.push("--debug".into());
        }
        crate::config::Config::from_args(args).expect("validated settings")
    }
}
fn invalid() -> io::Error {
    io::Error::other(
        "Invalid settings.ini. Preserve the file; use version=1, port=1..65535, timeout_ms=100..1000, discovery=true/false, log_level=info/debug, exactly once each.",
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn settings_save_is_atomic_and_invalid_file_is_preserved() {
        let dir = std::env::temp_dir().join(format!(
            "vg-settings-test-{}",
            crate::session::random_id().unwrap()
        ));
        fs::create_dir(&dir).unwrap();
        let mut value = DesktopSettings::load(&dir).unwrap();
        value.port = 9000;
        value.save(&dir).unwrap();
        assert_eq!(DesktopSettings::load(&dir).unwrap(), value);
        fs::write(dir.join("settings.ini"), b"version=2").unwrap();
        assert!(DesktopSettings::load(&dir).is_err());
        assert_eq!(fs::read(dir.join("settings.ini")).unwrap(), b"version=2");
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn settings_round_trip_bounds_and_unknown_versions() {
        let s = DesktopSettings::default();
        assert_eq!(DesktopSettings::parse(&s.text()).unwrap(), s);
        for text in [
            s.text().replace("version=1", "version=2"),
            s.text().replace("port=8765", "port=0"),
            s.text().replace("timeout_ms=250", "timeout_ms=99"),
            s.text() + "port=9000\n",
            s.text() + "state_rate=120\n",
            s.text().replace("discovery=true", "discovery=maybe"),
        ] {
            assert!(DesktopSettings::parse(&text).is_err());
        }
    }
}
