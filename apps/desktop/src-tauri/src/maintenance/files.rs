//! Maintenance touches named app-owned roots only; never the live DB or lock.
use super::MaintenanceAction;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(super) fn roots(
    app_data: &Path,
    log_dir: Option<&Path>,
    action: MaintenanceAction,
) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if action != MaintenanceAction::ResetPreferences {
        for name in ["recordings", "processing", "playback", "recovery"] {
            roots.push(app_data.join(name));
        }
    }
    if action == MaintenanceAction::FactoryReset {
        roots.push(app_data.join("models"));
        roots.push(app_data.join("logs"));
        if let Some(log_dir) = log_dir {
            if !roots.iter().any(|root| root == log_dir) {
                roots.push(log_dir.to_path_buf());
            }
        }
    }
    roots
}
pub(super) fn inventory(roots: &[PathBuf]) -> Result<(u64, u64, String), String> {
    let mut count = 0;
    let mut bytes = 0;
    let mut hash = Sha256::new();
    for root in roots {
        inventory_path(root, &mut count, &mut bytes, &mut hash)?;
    }
    Ok((count, bytes, hex::encode(hash.finalize())))
}
fn inventory_path(
    path: &Path,
    count: &mut u64,
    bytes: &mut u64,
    hash: &mut Sha256,
) -> Result<(), String> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err("Unable to inspect app-owned cleanup files.".into()),
    };
    if metadata.file_type().is_symlink() {
        return Err(
            "App storage contains a symbolic link. Remove that link before maintenance.".into(),
        );
    }
    hash.update(path.as_os_str().as_encoded_bytes());
    hash.update(metadata.len().to_le_bytes());
    if let Ok(modified) = metadata.modified() {
        if let Ok(time) = modified.duration_since(std::time::UNIX_EPOCH) {
            hash.update(time.as_nanos().to_le_bytes());
        }
    }
    if metadata.is_dir() {
        let mut entries = fs::read_dir(path)
            .map_err(|_| "Unable to inspect app-owned cleanup directory.")?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| "Unable to inspect app-owned cleanup directory.")?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            inventory_path(&entry.path(), count, bytes, hash)?;
        }
    } else if metadata.is_file() {
        *count += 1;
        *bytes += metadata.len();
    } else {
        return Err("App storage contains an unsupported file type.".into());
    }
    Ok(())
}
pub(super) fn remove_roots(roots: &[PathBuf]) -> Result<(), String> {
    // Inspect again immediately before deletion, including every descendant.
    inventory(roots)?;
    for root in roots {
        match fs::symlink_metadata(root) {
            Ok(metadata) if metadata.is_dir() => fs::remove_dir_all(root).map_err(|_| {
                "Unable to remove app-owned files. Cleanup is incomplete; retry maintenance."
            })?,
            Ok(_) => return Err("App-owned cleanup root is not a directory.".into()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err("Unable to inspect app-owned cleanup root.".into()),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deleting_named_roots_preserves_database_and_unknown_siblings() {
        let root =
            std::env::temp_dir().join(format!("daydream-maintenance-{}", uuid::Uuid::now_v7()));
        fs::create_dir_all(root.join("recordings")).unwrap();
        fs::write(root.join("recordings/private"), b"synthetic").unwrap();
        fs::write(root.join("daydream.sqlite3"), b"db").unwrap();
        fs::write(root.join("unowned"), b"keep").unwrap();
        remove_roots(&roots(&root, None, MaintenanceAction::ClearLibrary)).unwrap();
        assert!(root.join("daydream.sqlite3").exists());
        assert!(root.join("unowned").exists());
        assert!(!root.join("recordings").exists());
        remove_roots(&roots(&root, None, MaintenanceAction::ClearLibrary)).unwrap();
        fs::remove_dir_all(root).unwrap();
    }
    #[cfg(unix)]
    #[test]
    fn symlinks_never_traverse_outside_storage() {
        let root = std::env::temp_dir().join(format!(
            "daydream-maintenance-link-{}",
            uuid::Uuid::now_v7()
        ));
        fs::create_dir_all(root.join("recordings")).unwrap();
        std::os::unix::fs::symlink("/tmp", root.join("recordings/link")).unwrap();
        assert!(remove_roots(&roots(&root, None, MaintenanceAction::ClearLibrary)).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
