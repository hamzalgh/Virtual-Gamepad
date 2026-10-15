//! Fixed, validated local records and a bounded worker, separate from UDP admission.
use crate::protocol::InstanceId;
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    num::NonZeroU64,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PairingRecord {
    pub server: InstanceId,
    pub client: Option<InstanceId>,
}
impl PairingRecord {
    pub fn encode(self) -> [u8; 24] {
        let mut bytes = [0; 24];
        bytes[..8].copy_from_slice(b"VGPAIR\x01\0");
        bytes[8..16].copy_from_slice(&self.server.0.get().to_le_bytes());
        bytes[16..24].copy_from_slice(&self.client.map_or(0, |v| v.0.get()).to_le_bytes());
        bytes
    }
    pub fn decode(bytes: &[u8]) -> io::Result<Self> {
        if bytes.len() != 24 || &bytes[..8] != b"VGPAIR\x01\0" {
            return Err(io::Error::other(
                "Invalid pairing record; preserve it and restore a valid backup",
            ));
        }
        let server = NonZeroU64::new(u64::from_le_bytes(bytes[8..16].try_into().unwrap()))
            .ok_or_else(|| io::Error::other("Invalid server identity"))?;
        let client =
            NonZeroU64::new(u64::from_le_bytes(bytes[16..24].try_into().unwrap())).map(InstanceId);
        Ok(Self {
            server: InstanceId(server),
            client,
        })
    }
}

pub struct PairingStore {
    directory: PathBuf,
    _lock: File,
}
pub(crate) fn reject_link(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                if metadata.file_attributes() & 0x400 != 0 {
                    return Err(io::Error::other("Pairing path must not be a reparse point"));
                }
            }
            if metadata.file_type().is_symlink() {
                return Err(io::Error::other("Pairing path must not be a symbolic link"));
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    Ok(())
}
impl PairingStore {
    /// Duplicate our already owned OS handle; retain exclusive ownership across Stop.
    pub(crate) fn duplicate_owner(&self) -> io::Result<Self> {
        Ok(Self {
            directory: self.directory.clone(),
            _lock: self._lock.try_clone()?,
        })
    }
    pub(crate) fn reload(&self) -> io::Result<PairingRecord> {
        let path = self.directory.join("pairing.bin");
        reject_link(&path)?;
        let file = File::open(path)?;
        let mut bytes = Vec::new();
        file.take(25).read_to_end(&mut bytes)?;
        PairingRecord::decode(&bytes)
    }
    pub fn open(directory: PathBuf) -> io::Result<(Self, PairingRecord)> {
        reject_link(&directory)?;
        fs::create_dir_all(&directory)?;
        reject_link(&directory)?;
        #[cfg(windows)]
        crate::windows_storage::protect_directory(&directory)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))?;
        }
        let lock_path = directory.join("owner.lock");
        reject_link(&lock_path)?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)?;
        #[cfg(windows)]
        crate::windows_storage::protect_directory(&lock_path)?;
        lock.try_lock()
            .map_err(|_| io::Error::other("Another agent owns the pairing store"))?;
        let store = Self {
            directory,
            _lock: lock,
        };
        let path = store.directory.join("pairing.bin");
        reject_link(&path)?;
        let record = match File::open(&path) {
            Ok(file) => {
                #[cfg(windows)]
                crate::windows_storage::protect_directory(&path)?;
                let mut bytes = Vec::new();
                file.take(25).read_to_end(&mut bytes)?;
                PairingRecord::decode(&bytes)?
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let server = crate::session::random_id()
                    .ok_or_else(|| io::Error::other("OS randomness unavailable"))?;
                let record = PairingRecord {
                    server: InstanceId(server),
                    client: None,
                };
                store.save(record)?;
                record
            }
            Err(error) => return Err(error),
        };
        Ok((store, record))
    }
    pub fn save(&self, record: PairingRecord) -> io::Result<()> {
        let path = self.directory.join("pairing.bin");
        reject_link(&path)?;
        let random = crate::session::random_id()
            .ok_or_else(|| io::Error::other("OS randomness unavailable"))?;
        let temp = self
            .directory
            .join(format!("record-{:016x}.tmp", random.get()));
        let result = (|| {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp)?;
            file.write_all(&record.encode())?;
            file.sync_all()?;
            drop(file);
            fs::rename(&temp, &path)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temp);
        }
        result
    }
}

pub struct StoreRequest {
    pub record: PairingRecord,
    pub result: tokio::sync::oneshot::Sender<io::Result<()>>,
}
pub fn worker(
    store: PairingStore,
) -> io::Result<(
    std::sync::mpsc::SyncSender<StoreRequest>,
    std::thread::JoinHandle<()>,
)> {
    let (tx, rx) = std::sync::mpsc::sync_channel::<StoreRequest>(1);
    let task = std::thread::Builder::new()
        .name("pairing-storage".into())
        .spawn(move || {
            for request in rx {
                let _ = request.result.send(store.save(request.record));
            }
        })?;
    Ok((tx, task))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn corrupt_and_unknown_records_fail_closed() {
        let r = PairingRecord {
            server: InstanceId(NonZeroU64::new(1).unwrap()),
            client: Some(InstanceId(NonZeroU64::new(2).unwrap())),
        };
        assert!(PairingRecord::decode(&r.encode()).unwrap() == r);
        for length in [0, 8, 23, 25] {
            assert!(PairingRecord::decode(&vec![0; length]).is_err());
        }
        let mut b = r.encode();
        b[6] = 2;
        assert!(PairingRecord::decode(&b).is_err());
        b = r.encode();
        b[8..16].fill(0);
        assert!(PairingRecord::decode(&b).is_err());
    }
    #[test]
    fn retained_owner_survives_worker_clone_and_reload_never_recreates() {
        let dir = std::env::temp_dir().join(format!(
            "vg-owner-test-{}",
            crate::session::random_id().unwrap()
        ));
        let (owner, record) = PairingStore::open(dir.clone()).unwrap();
        let worker = owner.duplicate_owner().unwrap();
        drop(worker);
        assert!(PairingStore::open(dir.clone()).is_err());
        assert!(owner.reload().unwrap() == record);
        fs::write(dir.join("pairing.bin"), b"corrupt").unwrap();
        assert!(owner.reload().is_err());
        assert_eq!(fs::read(dir.join("pairing.bin")).unwrap(), b"corrupt");
        fs::remove_file(dir.join("pairing.bin")).unwrap();
        assert!(owner.reload().is_err());
        assert!(!dir.join("pairing.bin").exists());
        drop(owner);
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn disk_restart_rotation_lock_and_corruption() {
        let dir = std::env::temp_dir().join(format!(
            "vg-pair-test-{}",
            crate::session::random_id().unwrap()
        ));
        let (store, first) = PairingStore::open(dir.clone()).unwrap();
        assert!(PairingStore::open(dir.clone()).is_err());
        let paired = PairingRecord {
            client: Some(InstanceId(NonZeroU64::new(99).unwrap())),
            ..first
        };
        store.save(paired).unwrap();
        drop(store);
        let (store, second) = PairingStore::open(dir.clone()).unwrap();
        assert!(paired == second);
        store.save(first).unwrap();
        drop(store);
        let (store, third) = PairingStore::open(dir.clone()).unwrap();
        assert!(third == first);
        drop(store);
        fs::write(dir.join("pairing.bin"), b"corrupt").unwrap();
        assert!(PairingStore::open(dir.clone()).is_err());
        fs::remove_dir_all(dir).unwrap();
    }
}
