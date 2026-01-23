use std::path::Path;

use crate::CommandError;

/// Create a new `git symbolic-ref` command builder.
#[must_use]
pub fn new() -> SymbolicRef<'static> {
    SymbolicRef::new()
}

/// Builder for `git symbolic-ref` command.
///
/// See `git symbolic-ref --help` for full documentation.
#[derive(Debug)]
pub struct SymbolicRef<'a> {
    repo_path: Option<&'a Path>,
    quiet: bool,
    short: bool,
    name: Option<&'a str>,
}

impl<'a> SymbolicRef<'a> {
    #[must_use]
    fn new() -> Self {
        Self {
            repo_path: None,
            quiet: false,
            short: false,
            name: None,
        }
    }

    /// Set the repository path (`-C <path>`).
    #[must_use]
    pub fn repo_path(mut self, path: &'a Path) -> Self {
        self.repo_path = Some(path);
        self
    }

    crate::flag_methods! {
        /// Suppress error messages if the symbolic ref does not exist.
        ///
        /// Corresponds to `--quiet`.
        pub fn quiet / quiet_if, quiet, "Conditionally suppress error messages."
    }

    crate::flag_methods! {
        /// Output in short form (e.g., `main` instead of `refs/heads/main`).
        ///
        /// Corresponds to `--short`.
        pub fn short / short_if, short, "Conditionally output in short form."
    }

    /// Set the symbolic ref name to read.
    #[must_use]
    pub fn name(mut self, name: &'a str) -> Self {
        self.name = Some(name);
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

    /// Execute and return full output regardless of exit status.
    ///
    /// Use this when you need to inspect stderr on failure.
    pub fn output(self) -> Result<cmd_proc::Output, CommandError> {
        self.build().output()
    }

    fn build(self) -> cmd_proc::Command {
        crate::base_command(self.repo_path)
            .argument("symbolic-ref")
            .optional_argument(self.quiet.then_some("--quiet"))
            .optional_argument(self.short.then_some("--short"))
            .optional_argument(self.name)
    }
}

impl Default for SymbolicRef<'_> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "test-utils")]
impl SymbolicRef<'_> {
    /// Compare the built command with another command using debug representation.
    pub fn test_eq(&self, other: &cmd_proc::Command) {
        let command = Self {
            repo_path: self.repo_path,
            quiet: self.quiet,
            short: self.short,
            name: self.name,
        }
        .build();
        command.test_eq(other);
    }
}
