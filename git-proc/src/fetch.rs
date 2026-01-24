use std::path::Path;

use crate::CommandError;
use crate::url::Remote;

/// Create a new `git fetch` command builder.
#[must_use]
pub fn new() -> Fetch<'static> {
    Fetch::new()
}

/// Builder for `git fetch` command.
///
/// See `git fetch --help` for full documentation.
#[derive(Debug)]
pub struct Fetch<'a> {
    repo_path: Option<&'a Path>,
    all: bool,
    prune: bool,
    quiet: bool,
    progress: bool,
    remote: Option<&'a Remote>,
}

impl<'a> Fetch<'a> {
    #[must_use]
    fn new() -> Self {
        Self {
            repo_path: None,
            all: false,
            prune: false,
            quiet: false,
            progress: false,
            remote: None,
        }
    }

    /// Set the repository path (`-C <path>`).
    #[must_use]
    pub fn repo_path(mut self, path: &'a Path) -> Self {
        self.repo_path = Some(path);
        self
    }

    crate::flag_methods! {
        /// Fetch all remotes.
        ///
        /// Corresponds to `--all`.
        pub fn all / all_if, all, "Conditionally fetch all remotes."
    }

    crate::flag_methods! {
        /// Prune remote-tracking branches that no longer exist.
        ///
        /// Corresponds to `--prune`.
        pub fn prune / prune_if, prune, "Conditionally prune remote-tracking branches."
    }

    crate::flag_methods! {
        /// Suppress progress output.
        ///
        /// Corresponds to `--quiet`.
        pub fn quiet / quiet_if, quiet, "Conditionally suppress progress output."
    }

    crate::flag_methods! {
        /// Force progress output even when stderr is not a terminal.
        ///
        /// Corresponds to `--progress`.
        pub fn progress / progress_if, progress, "Conditionally force progress output."
    }

    /// Set the remote to fetch from.
    #[must_use]
    pub fn remote(mut self, remote: &'a Remote) -> Self {
        self.remote = Some(remote);
        self
    }

    /// Execute the command and return the exit status.
    pub fn status(self) -> Result<(), CommandError> {
        crate::Build::build(self).status()
    }

    /// Spawn the command for long-running operations.
    ///
    /// Returns a spawned process that can be run and waited on.
    #[must_use]
    pub fn spawn(self) -> cmd_proc::Spawn {
        crate::Build::build(self).spawn()
    }
}

impl Default for Fetch<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::Build for Fetch<'_> {
    fn build(self) -> cmd_proc::Command {
        crate::base_command(self.repo_path)
            .argument("fetch")
            .optional_argument(self.all.then_some("--all"))
            .optional_argument(self.prune.then_some("--prune"))
            .optional_argument(self.quiet.then_some("--quiet"))
            .optional_argument(self.progress.then_some("--progress"))
            .optional_argument(self.remote)
    }
}

#[cfg(feature = "test-utils")]
impl Fetch<'_> {
    /// Compare the built command with another command using debug representation.
    pub fn test_eq(&self, other: &cmd_proc::Command) {
        let command = crate::Build::build(Self {
            repo_path: self.repo_path,
            all: self.all,
            prune: self.prune,
            quiet: self.quiet,
            progress: self.progress,
            remote: self.remote,
        });
        command.test_eq(other);
    }
}
