#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitUrl(String);

impl GitUrl {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Extract repository name from a Git URL
    ///
    /// Handles various Git URL formats:
    /// - SSH: git@github.com:user/repo.git -> repo
    /// - HTTPS: https://github.com/user/repo.git -> repo
    /// - SSH with protocol: ssh://git@github.com/user/repo.git -> repo
    ///
    /// Returns the repository name without the .git extension if present
    #[must_use]
    pub fn extract_repo_name(&self) -> String {
        let url = self.0.as_str();

        // First, extract the path portion of the URL
        let path = if let Some(colon_pos) = url.rfind(':') {
            // SSH format: git@github.com:user/repo.git
            // Check if this is actually a port number (has // before it)
            if url[..colon_pos].contains("//") {
                // This is ssh://git@github.com/user/repo.git format
                // Find the path after the last ://
                if let Some(double_slash_pos) = url.rfind("://") {
                    &url[double_slash_pos + 3..]
                } else {
                    url
                }
            } else {
                // This is git@github.com:user/repo.git format
                &url[colon_pos + 1..]
            }
        } else {
            // HTTPS or other format - find path after ://
            if let Some(double_slash_pos) = url.rfind("://") {
                &url[double_slash_pos + 3..]
            } else {
                url
            }
        };

        // Now extract just the last component (repo name)
        let repo_with_maybe_git = path.rsplit('/').next().unwrap_or(path);

        // Remove .git suffix if present
        let name = repo_with_maybe_git
            .strip_suffix(".git")
            .unwrap_or(repo_with_maybe_git);

        name.to_string()
    }
}

impl std::fmt::Display for GitUrl {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

impl AsRef<std::ffi::OsStr> for GitUrl {
    fn as_ref(&self) -> &std::ffi::OsStr {
        self.0.as_ref()
    }
}

impl std::str::FromStr for GitUrl {
    type Err = GitUrlError;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        if string.is_empty() {
            return Err(GitUrlError::Empty);
        }

        Ok(Self(string.to_string()))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GitUrlError {
    #[error("Git URL cannot be empty")]
    Empty,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_url() {
        assert!("git@github.com:user/repo.git".parse::<GitUrl>().is_ok());
        assert!("https://github.com/user/repo.git".parse::<GitUrl>().is_ok());
    }

    #[test]
    fn test_empty() {
        assert!(matches!("".parse::<GitUrl>(), Err(GitUrlError::Empty)));
    }

    #[test]
    fn test_extract_repo_name_ssh_with_git_suffix() {
        let url: GitUrl = "git@github.com:user/repo.git".parse().unwrap();
        assert_eq!(url.extract_repo_name(), "repo");
    }

    #[test]
    fn test_extract_repo_name_ssh_without_git_suffix() {
        let url: GitUrl = "git@github.com:user/repo".parse().unwrap();
        assert_eq!(url.extract_repo_name(), "repo");
    }

    #[test]
    fn test_extract_repo_name_https_with_git_suffix() {
        let url: GitUrl = "https://github.com/user/repo.git".parse().unwrap();
        assert_eq!(url.extract_repo_name(), "repo");
    }

    #[test]
    fn test_extract_repo_name_https_without_git_suffix() {
        let url: GitUrl = "https://github.com/user/repo".parse().unwrap();
        assert_eq!(url.extract_repo_name(), "repo");
    }

    #[test]
    fn test_extract_repo_name_ssh_protocol_with_git_suffix() {
        let url: GitUrl = "ssh://git@github.com/user/repo.git".parse().unwrap();
        assert_eq!(url.extract_repo_name(), "repo");
    }

    #[test]
    fn test_extract_repo_name_gitlab() {
        let url: GitUrl = "git@gitlab.com:user/repo.git".parse().unwrap();
        assert_eq!(url.extract_repo_name(), "repo");
    }

    #[test]
    fn test_extract_repo_name_nested_path() {
        let url: GitUrl = "git@github.com:org/team/repo.git".parse().unwrap();
        assert_eq!(url.extract_repo_name(), "repo");
    }
}
