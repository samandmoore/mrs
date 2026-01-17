use crate::{Command, Config, Error, GitUrl, RepoName};

#[derive(Debug, clap::Parser)]
pub struct Setup {
    /// Git remote URL to clone
    url: GitUrl,

    /// Local name for the repository (defaults to name extracted from URL)
    #[clap(long)]
    repo: Option<RepoName>,
}

impl Setup {
    pub fn run(self, config: &Config) -> Result<(), Error> {
        // Determine repo name: use provided or extract from URL
        let repo = if let Some(repo_name) = self.repo {
            repo_name
        } else {
            // Extract repo name from URL and parse it as a RepoName
            self.url.extract_repo_name().parse::<RepoName>()?
        };

        let bare_path = config.bare_repo_path(&repo);
        let worktree_base = config.worktree_base_path(&repo);

        if bare_path.exists() {
            return Err(Error::RepoAlreadyExists(repo));
        }

        log::info!("Cloning bare repository to {}", bare_path.display());

        Command::new("git")
            .argument("clone")
            .argument("--bare")
            .argument(&self.url)
            .argument(&bare_path)
            .status()?;

        log::info!("Configuring remote tracking branches");

        Command::new("git")
            .argument("-C")
            .argument(&bare_path)
            .argument("config")
            .argument("remote.origin.fetch")
            .argument("+refs/heads/*:refs/remotes/origin/*")
            .status()?;

        Command::new("git")
            .argument("-C")
            .argument(&bare_path)
            .argument("fetch")
            .argument("origin")
            .status()?;

        log::info!("Creating worktree directory {}", worktree_base.display());

        std::fs::create_dir_all(&worktree_base)?;

        log::info!("Setup complete for repository '{}'", repo);

        Ok(())
    }
}
