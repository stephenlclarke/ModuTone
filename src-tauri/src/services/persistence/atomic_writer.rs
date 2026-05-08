// Phase: 2
// Atomic file writer: write to .tmp, then rename to target path.
// If the process crashes mid-write, only the .tmp file is affected;
// the original file remains intact.

use std::fs;
use std::io::Write;
use std::path::Path;

use crate::contracts::errors::IpcError;

/// Write `content` to `path` atomically.
///
/// Strategy: write to `<path>.tmp`, flush + sync, then rename over the target.
/// On failure the .tmp file is cleaned up on a best-effort basis.
pub fn atomic_write(path: &Path, content: &[u8]) -> Result<(), IpcError> {
    let tmp_path = path.with_extension("tmp");

    let result = (|| -> Result<(), std::io::Error> {
        let mut file = fs::File::create(&tmp_path)?;
        file.write_all(content)?;
        file.sync_all()?;
        fs::rename(&tmp_path, path)?;
        Ok(())
    })();

    match result {
        Ok(()) => Ok(()),
        Err(e) => {
            // Best-effort cleanup of the temp file
            let _ = fs::remove_file(&tmp_path);
            Err(IpcError {
                code: "STORE_WRITE_FAILED".to_string(),
                message: "Failed to write metadata file".to_string(),
                detail: Some(e.to_string()),
                subsystem: "persistence".to_string(),
            })
        }
    }
}
