use std::path::Path;

use crate::CommandError;

/// Create a new `git merge-base` command builder.
#[must_use]
pub fn new() -> MergeBase<'static> {
    MergeBase::new()
}

/// Builder for `git merge-base` command.
///
/// See `git merge-base --help` for full documentation.
#[derive(Debug)]
pub struct MergeBase<'a> {
    repo_path: Option<&'a Path>,
    is_ancestor: bool,
    commit1: Option<&'a str>,
    commit2: Option<&'a str>,
}

impl<'a> MergeBase<'a> {
    #[must_use]
    fn new() -> Self {
        Self {
            repo_path: None,
            is_ancestor: false,
            commit1: None,
            commit2: None,
        }
    }

    /// Set the repository path (`-C <path>`).
    #[must_use]
    pub fn repo_path(mut self, path: &'a Path) -> Self {
        self.repo_path = Some(path);
        self
    }

    crate::flag_methods! {
        /// Check if commit1 is an ancestor of commit2.
        ///
        /// Corresponds to `--is-ancestor`.
        pub fn is_ancestor / is_ancestor_if, is_ancestor, "Conditionally check if commit1 is an ancestor of commit2."
    }

    /// Set the first commit.
    #[must_use]
    pub fn commit1(mut self, commit: &'a str) -> Self {
        self.commit1 = Some(commit);
        self
    }

    /// Set the second commit.
    #[must_use]
    pub fn commit2(mut self, commit: &'a str) -> Self {
        self.commit2 = Some(commit);
        self
    }

    /// Execute the command and return the exit status.
    pub fn status(self) -> Result<(), CommandError> {
        self.build().status()
    }

    /// Capture stdout from this command.
    #[must_use]
    pub fn stdout(self) -> cmd_proc::Capture {
        self.build().stdout()
    }

    fn build(self) -> cmd_proc::Command {
        crate::base_command(self.repo_path)
            .argument("merge-base")
            .optional_argument(self.is_ancestor.then_some("--is-ancestor"))
            .optional_argument(self.commit1)
            .optional_argument(self.commit2)
    }
}

impl Default for MergeBase<'_> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "test-utils")]
impl MergeBase<'_> {
    /// Compare the built command with another command using debug representation.
    pub fn test_eq(&self, other: &cmd_proc::Command) {
        let command = Self {
            repo_path: self.repo_path,
            is_ancestor: self.is_ancestor,
            commit1: self.commit1,
            commit2: self.commit2,
        }
        .build();
        command.test_eq(other);
    }
}
