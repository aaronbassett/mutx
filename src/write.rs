use crate::error::{MutxError, Result};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy)]
pub enum WriteMode {
    Simple,
    Streaming,
}

pub struct AtomicWriter {
    mode: WriteMode,
    target: PathBuf,
    buffer: Vec<u8>,
    temp_file: Option<atomic_write_file::AtomicWriteFile>,
}

impl AtomicWriter {
    /// Create a new atomic writer for the target file
    pub fn new(target: &Path, mode: WriteMode) -> Result<Self> {
        Ok(AtomicWriter {
            mode,
            target: target.to_path_buf(),
            buffer: Vec::new(),
            temp_file: None,
        })
    }

    /// Write data (buffered in simple mode)
    pub fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        match self.mode {
            WriteMode::Simple => {
                self.buffer.extend_from_slice(buf);
                Ok(())
            }
            WriteMode::Streaming => {
                // Initialize temp file on first write
                if self.temp_file.is_none() {
                    self.temp_file = Some(
                        atomic_write_file::AtomicWriteFile::open(&self.target).map_err(|e| {
                            MutxError::WriteFailed {
                                path: self.target.clone(),
                                source: e,
                            }
                        })?,
                    );
                }

                if let Some(temp) = self.temp_file.as_mut() {
                    temp.write_all(buf).map_err(|e| MutxError::WriteFailed {
                        path: self.target.clone(),
                        source: e,
                    })?;
                }
                Ok(())
            }
        }
    }

    /// Commit the write (atomic rename)
    pub fn commit(mut self) -> Result<()> {
        match self.mode {
            WriteMode::Simple => {
                let mut temp =
                    atomic_write_file::AtomicWriteFile::open(&self.target).map_err(|e| {
                        MutxError::WriteFailed {
                            path: self.target.clone(),
                            source: e,
                        }
                    })?;

                temp.write_all(&self.buffer)
                    .map_err(|e| MutxError::WriteFailed {
                        path: self.target.clone(),
                        source: e,
                    })?;

                temp.commit().map_err(|e| MutxError::WriteFailed {
                    path: self.target.clone(),
                    source: e,
                })?;
            }
            WriteMode::Streaming => {
                if let Some(temp) = self.temp_file.take() {
                    temp.commit().map_err(|e| MutxError::WriteFailed {
                        path: self.target.clone(),
                        source: e,
                    })?;
                } else {
                    // No writes happened, create empty file
                    let temp =
                        atomic_write_file::AtomicWriteFile::open(&self.target).map_err(|e| {
                            MutxError::WriteFailed {
                                path: self.target.clone(),
                                source: e,
                            }
                        })?;
                    temp.commit().map_err(|e| MutxError::WriteFailed {
                        path: self.target.clone(),
                        source: e,
                    })?;
                }
            }
        }
        Ok(())
    }
}
