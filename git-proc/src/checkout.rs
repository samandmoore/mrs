use std::path::Path;

use crate::CommandError;

/// Create a new `git checkout` command builder.
#[must_use]
pub fn new() -> Checkout<'static> {
    Checkout::new()
}

/// Builder for `git checkout` command.
///
/// See `git checkout --help` for full documentation.
#[derive(Debug)]
pub struct Checkout<'a> {
    repo_path: Option<&'a Path>,
    quiet: bool,
    branch: Option<&'a str>,
}

impl<'a> Checkout<'a> {
    #[must_use]
    fn new() -> Self {
        Self {
            repo_path: None,
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
        /// Suppress feedback messages.
        ///
        /// Corresponds to `--quiet`.
        pub fn quiet / quiet_if, quiet, "Conditionally suppress feedback messages."
    }

    /// Set the branch to checkout.
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
            .argument("checkout")
            .optional_argument(self.quiet.then_some("--quiet"))
            .optional_argument(self.branch)
    }
}

impl Default for Checkout<'_> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "test-utils")]
impl Checkout<'_> {
    /// Compare the built command with another command using debug representation.
    pub fn test_eq(&self, other: &cmd_proc::Command) {
        let command = Self {
            repo_path: self.repo_path,
            quiet: self.quiet,
            branch: self.branch,
        }
        .build();
        command.test_eq(other);
    }
}
