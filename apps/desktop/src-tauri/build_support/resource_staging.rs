use std::{
    fs::{self, File},
    io::{self, Read},
    path::Path,
};

/// Avoid changing watched resource timestamps when a build produced identical bytes.
pub fn copy_if_changed(source: &Path, destination: &Path) -> io::Result<()> {
    if !same_contents(source, destination)? {
        fs::copy(source, destination)?;
    }
    // Repair executable permissions without rewriting an unchanged binary.
    let permissions = fs::metadata(source)?.permissions();
    if fs::metadata(destination)?.permissions() != permissions {
        fs::set_permissions(destination, permissions)?;
    }
    Ok(())
}

fn same_contents(source: &Path, destination: &Path) -> io::Result<bool> {
    let mut source = File::open(source)?;
    let mut destination = match File::open(destination) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    if source.metadata()?.len() != destination.metadata()?.len() {
        return Ok(false);
    }
    let mut source_buffer = [0; 64 * 1024];
    let mut destination_buffer = [0; 64 * 1024];
    loop {
        let count = source.read(&mut source_buffer)?;
        destination.read_exact(&mut destination_buffer[..count])?;
        if source_buffer[..count] != destination_buffer[..count] {
            return Ok(false);
        }
        if count == 0 {
            return Ok(true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::copy_if_changed;
    use std::{fs, path::PathBuf, time::SystemTime};

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir()
                .join(format!("daydream-staging-{}-{nonce}", std::process::id()));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn repeated_staging_preserves_timestamp_and_replaces_changed_bytes() {
        let fixture = Fixture::new();
        let source = fixture.0.join("worker");
        let destination = fixture.0.join("staged-worker");
        // Cross a comparison-buffer boundary, including a short final chunk.
        let contents = vec![17; 64 * 1024 + 3];
        fs::write(&source, &contents).unwrap();
        copy_if_changed(&source, &destination).unwrap();
        let sentinel = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1);
        fs::OpenOptions::new()
            .write(true)
            .open(&destination)
            .unwrap()
            .set_modified(sentinel)
            .unwrap();
        copy_if_changed(&source, &destination).unwrap();
        assert_eq!(
            fs::metadata(&destination).unwrap().modified().unwrap(),
            sentinel
        );
        let mut changed = contents;
        changed[64 * 1024 + 2] = 23;
        fs::write(&source, &changed).unwrap();
        copy_if_changed(&source, &destination).unwrap();
        assert_eq!(fs::read(&destination).unwrap(), changed);
    }

    #[test]
    fn missing_source_does_not_damage_existing_resource() {
        let fixture = Fixture::new();
        let destination = fixture.0.join("worker");
        fs::write(&destination, b"working resource").unwrap();
        assert!(copy_if_changed(&fixture.0.join("missing"), &destination).is_err());
        assert_eq!(fs::read(destination).unwrap(), b"working resource");
    }

    #[cfg(unix)]
    #[test]
    fn unchanged_executable_permissions_are_repaired_without_rewriting() {
        use std::os::unix::fs::PermissionsExt;
        let fixture = Fixture::new();
        let source = fixture.0.join("worker");
        let destination = fixture.0.join("staged-worker");
        fs::write(&source, b"worker").unwrap();
        fs::set_permissions(&source, fs::Permissions::from_mode(0o755)).unwrap();
        copy_if_changed(&source, &destination).unwrap();
        fs::set_permissions(&destination, fs::Permissions::from_mode(0o644)).unwrap();
        let modified = fs::metadata(&destination).unwrap().modified().unwrap();
        copy_if_changed(&source, &destination).unwrap();
        assert_eq!(
            fs::metadata(&destination).unwrap().permissions().mode() & 0o777,
            0o755
        );
        assert_eq!(
            fs::metadata(&destination).unwrap().modified().unwrap(),
            modified
        );
    }
}
