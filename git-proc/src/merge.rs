use std::path::Path;

use crate::CommandError;

/// Create a new `git merge` command builder.
#[must_use]
pub fn new() -> Merge<'static> {
    Merge::new()
}

/// Builder for `git merge` command.
///
/// See `git merge --help` for full documentation.
#[derive(Debug)]
pub struct Merge<'a> {
    repo_path: Option<&'a Path>,
    ff_only: bool,
    quiet: bool,
    branch: Option<&'a str>,
}

impl<'a> Merge<'a> {
    #[must_use]
    fn new() -> Self {
        Self {
            repo_path: None,
            ff_only: false,
            quiet: false,
            branch: None,
        }
    }

    /// Set the repository path (`-C <path>`).
    #[must_use]
    pub fn repo_path(mut self, path: &'a Path) -> Self {
        self.repo_path = Some(path);
        self
    }

    crate::flag_methods! {
        /// Only allow fast-forward merges.
        ///
        /// Corresponds to `--ff-only`.
        pub fn ff_only / ff_only_if, ff_only, "Conditionally allow only fast-forward merges."
    }

    crate::flag_methods! {
        /// Suppress merge output.
        ///
        /// Corresponds to `--quiet`.
        pub fn quiet / quiet_if, quiet, "Conditionally suppress merge output."
    }

    /// Set the branch to merge.
    #[must_use]
    pub fn branch(mut self, branch: &'a str) -> Self {
        self.branch = Some(branch);
        self
    }

    /// Execute the command and return the exit status.
    pub fn status(self) -> Result<(), CommandError> {
        self.build().status()
    }

    fn build(self) -> cmd_proc::Command {
        crate::base_command(self.repo_path)
            .argument("merge")
            .optional_argument(self.ff_only.then_some("--ff-only"))
            .optional_argument(self.quiet.then_some("--quiet"))
            .optional_argument(self.branch)
    }
}

impl Default for Merge<'_> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "test-utils")]
impl Merge<'_> {
    /// Compare the built command with another command using debug representation.
    pub fn test_eq(&self, other: &cmd_proc::Command) {
        let command = Self {
            repo_path: self.repo_path,
            ff_only: self.ff_only,
            quiet: self.quiet,
            branch: self.branch,
        }
        .build();
        command.test_eq(other);
    }
}
