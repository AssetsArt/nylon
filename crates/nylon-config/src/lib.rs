pub mod runtime;

use nylon_error::NylonError;
use std::path::PathBuf;

const MAX_DEPTH: u16 = 10;

fn read_dir_recursive(dir: &String, max_depth: u16) -> Result<Vec<PathBuf>, NylonError> {
    let mut files = Vec::new();
    let path_buf = PathBuf::from(dir);
    for entry in std::fs::read_dir(path_buf).map_err(|e| {
        NylonError::ConfigError(format!("Unable to read config directory {:?}: {}", dir, e))
    })? {
        let entry = entry.map_err(|e| {
            NylonError::ConfigError(format!(
                "Unable to read file in config directory {:?}: {}",
                dir, e
            ))
        })?;
        let path = entry.path();
        if path.is_dir() {
            if max_depth > 0 {
                files.append(&mut read_dir_recursive(
                    &path.to_string_lossy().to_string(),
                    max_depth - 1,
                )?);
            }
        } else {
            files.push(path);
        }
    }
    Ok(files)
}

pub fn read(dir: &String) -> Result<(), NylonError> {
    let _files = read_dir_recursive(dir, MAX_DEPTH)?;
    Ok(())
}
