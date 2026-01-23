use std::path::Path;

use crate::CommandError;
use crate::url::RemoteName;

/// Create a `git remote get-url` command builder.
#[must_use]
pub fn get_url(name: &RemoteName) -> Remote<'_> {
    Remote::get_url(name)
}

/// Create a `git remote` list command builder.
#[must_use]
pub fn list() -> Remote<'static> {
    Remote::list()
}

/// Builder for `git remote` command.
///
/// See `git remote --help` for full documentation.
#[derive(Debug)]
pub struct Remote<'a> {
    repo_path: Option<&'a Path>,
    subcommand: RemoteSubcommand<'a>,
}

#[derive(Debug)]
enum RemoteSubcommand<'a> {
    GetUrl { name: &'a RemoteName },
    List { verbose: bool },
}

impl<'a> Remote<'a> {
    #[must_use]
    fn get_url(name: &'a RemoteName) -> Self {
        Self {
            repo_path: None,
            subcommand: RemoteSubcommand::GetUrl { name },
        }
    }

    #[must_use]
    fn list() -> Remote<'static> {
        Remote {
            repo_path: None,
            subcommand: RemoteSubcommand::List { verbose: false },
        }
    }

    /// Show more information about remotes (only applies to list).
    ///
    /// Corresponds to `--verbose`.
    #[must_use]
    pub fn verbose(mut self) -> Self {
        if let RemoteSubcommand::List { ref mut verbose } = self.subcommand {
            *verbose = true;
        }
        self
    }

    /// Conditionally show more information about remotes.
    #[must_use]
    pub fn verbose_if(mut self, value: bool) -> Self {
        if let RemoteSubcommand::List { ref mut verbose } = self.subcommand {
            *verbose = value;
        }
        self
    }

    /// Set the repository path (`-C <path>`).
    #[must_use]
    pub fn repo_path(mut self, path: &'a Path) -> Self {
        self.repo_path = Some(path);
        self
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
        let cmd = crate::base_command(self.repo_path).argument("remote");

        match self.subcommand {
            RemoteSubcommand::GetUrl { name } => cmd.argument("get-url").argument(name),
            RemoteSubcommand::List { verbose } => {
                cmd.optional_argument(verbose.then_some("--verbose"))
            }
        }
    }
}

#[cfg(feature = "test-utils")]
impl Remote<'_> {
    /// Compare the built command with another command using debug representation.
    pub fn test_eq(&self, other: &cmd_proc::Command) {
        let command = Self {
            repo_path: self.repo_path,
            subcommand: match &self.subcommand {
                RemoteSubcommand::GetUrl { name } => RemoteSubcommand::GetUrl { name },
                RemoteSubcommand::List { verbose } => RemoteSubcommand::List { verbose: *verbose },
            },
        }
        .build();
        command.test_eq(other);
    }
}
