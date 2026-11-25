use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

/// A file or directory of a package that could not be written.
#[derive(Debug)]
pub struct WriteError {
    action: &'static str,
    path: PathBuf,
    source: io::Error,
}

impl WriteError {
    /// `action` is the verb of what failed at `path`.
    pub(super) fn new(action: &'static str, path: &Path, source: io::Error) -> Self {
        Self {
            action,
            path: path.to_owned(),
            source,
        }
    }
}

impl fmt::Display for WriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            action,
            path,
            source,
        } = self;
        write!(f, "cannot {action} '{}': {source}", path.display())
    }
}

impl std::error::Error for WriteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::path::Path;

    use super::WriteError;

    #[test]
    fn names_what_failed_and_where() {
        let source = io::Error::other("disk full");
        let error = WriteError::new("write", Path::new("fix44/enums.py"), source);

        assert_eq!(
            error.to_string(),
            "cannot write 'fix44/enums.py': disk full"
        );
    }
}
