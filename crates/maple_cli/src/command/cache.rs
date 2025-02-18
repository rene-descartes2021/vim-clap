use std::fs::read_dir;
use std::io::Write;
use std::path::{self, Path, PathBuf};

use anyhow::Result;
use clap::Parser;

use utility::{clap_cache_dir, remove_dir_contents};

use crate::datastore::CACHE_INFO_IN_MEMORY;

/// List and remove all the cached contents.
#[derive(Parser, Debug, Clone)]
pub struct Cache {
    /// List the current cached entries.
    #[clap(short, long)]
    list: bool,

    /// Purge all the cached contents.
    #[clap(short, long)]
    purge: bool,
}

// The cache directory is not huge and pretty deep, hence the recursive version is acceptable.
fn dir_size(path: impl Into<PathBuf>) -> std::io::Result<u64> {
    fn dir_size(mut dir: std::fs::ReadDir) -> std::io::Result<u64> {
        dir.try_fold(0, |acc, file| {
            let file = file?;
            let size = match file.metadata()? {
                data if data.is_dir() => dir_size(std::fs::read_dir(file.path())?)?,
                data => data.len(),
            };
            Ok(acc + size)
        })
    }

    dir_size(read_dir(path.into())?)
}

impl Cache {
    pub fn run(&self) -> Result<()> {
        let cache_dir = clap_cache_dir()?;
        if self.purge {
            if let Ok(cache_size) = dir_size(&cache_dir) {
                let readable_size = if cache_size > 1024 * 1024 {
                    format!("{}MB", cache_size / 1024 / 1024)
                } else if cache_size > 1024 {
                    format!("{}KB", cache_size / 1024)
                } else {
                    format!("{}B", cache_size)
                };
                println!("Cache size: {:?}", readable_size);
            }
            if let Some(f) = crate::datastore::CACHE_JSON_PATH.as_deref() {
                std::fs::remove_file(f)?;
                println!("Cache metadata {} has been deleted", f.display());
            }
            remove_dir_contents(&cache_dir)?;
            println!(
                "Current cache directory {} has been purged",
                cache_dir.display()
            );
            return Ok(());
        }
        if self.list {
            self.list(&cache_dir)?;
        }
        Ok(())
    }

    fn list(&self, cache_dir: &Path) -> Result<()> {
        let stdout = std::io::stdout();
        let mut lock = stdout.lock();

        let cache_dir_str = cache_dir.display();
        writeln!(lock, "Current cache directory:")?;
        writeln!(lock, "\t{}\n", cache_dir_str)?;

        let cache_info = CACHE_INFO_IN_MEMORY.lock();
        writeln!(lock, "{:#?}\n", cache_info)?;

        if self.list {
            writeln!(lock, "Cached entries:")?;
            let mut entries = read_dir(cache_dir)?
                .map(|res| {
                    res.map(|e| {
                        e.path()
                            .file_name()
                            .and_then(std::ffi::OsStr::to_str)
                            .map(Into::into)
                            .unwrap_or_else(|| panic!("Couldn't get file name from {:?}", e.path()))
                    })
                })
                .collect::<Result<Vec<String>, std::io::Error>>()?;

            entries.sort();

            for fname in entries {
                writeln!(lock, "\t{}{}{}", cache_dir_str, path::MAIN_SEPARATOR, fname)?;
            }
        }
        Ok(())
    }
}
