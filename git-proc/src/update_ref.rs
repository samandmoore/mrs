use std::path::Path;

use crate::CommandError;

/// Create a new `git update-ref` command builder.
#[must_use]
pub fn new() -> UpdateRef<'static> {
    UpdateRef::new()
}

/// Builder for `git update-ref` command.
///
/// See `git update-ref --help` for full documentation.
#[derive(Debug)]
pub struct UpdateRef<'a> {
    repo_path: Option<&'a Path>,
    reference: Option<&'a str>,
    newvalue: Option<&'a str>,
}

impl<'a> UpdateRef<'a> {
    #[must_use]
    fn new() -> Self {
        Self {
            repo_path: None,
            reference: None,
            newvalue: None,
        }
    }

    /// Set the repository path (`-C <path>`).
    #[must_use]
    pub fn repo_path(mut self, path: &'a Path) -> Self {
        self.repo_path = Some(path);
        self
    }

    /// Set the ref to update.
    ///
    /// This is a reference name such as:
    /// - `HEAD`: The current branch pointer
    /// - `refs/heads/main`: A branch reference
    /// - `refs/tags/v1.0`: A tag reference
    /// - `refs/remotes/origin/main`: A remote tracking branch
    #[must_use]
    pub fn reference(mut self, reference: &'a str) -> Self {
        self.reference = Some(reference);
        self
    }

    /// Set the new value for the ref.
    ///
    /// This is typically a commit SHA (40-character hex string) that the
    /// reference should point to. Can also be another ref name in some contexts.
    #[must_use]
    pub fn newvalue(mut self, newvalue: &'a str) -> Self {
        self.newvalue = Some(newvalue);
        self
    }

    /// Execute the command and return the exit status.
    pub fn status(self) -> Result<(), CommandError> {
        self.build().status()
    }

    fn build(self) -> cmd_proc::Command {
        crate::base_command(self.repo_path)
            .argument("update-ref")
            .optional_argument(self.reference)
            .optional_argument(self.newvalue)
    }
}

impl Default for UpdateRef<'_> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "test-utils")]
impl UpdateRef<'_> {
    /// Compare the built command with another command using debug representation.
    pub fn test_eq(&self, other: &cmd_proc::Command) {
        let command = Self {
            repo_path: self.repo_path,
            reference: self.reference,
            newvalue: self.newvalue,
        }
        .build();
        command.test_eq(other);
    }
}
