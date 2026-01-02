use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseInfo {
    pub tag_name: String,
    pub name: String,
    pub body: String,
    pub published_at: String,
    pub html_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub name: String,
    pub body: String,
    pub published_at: String,
    pub html_url: String,
}

#[derive(Clone)]
pub struct UpdateChecker {
    last_check: Option<SystemTime>,
    current_version: String,
}

impl UpdateChecker {
    pub fn new(current_version: String) -> Self {
        Self {
            last_check: None,
            current_version,
        }
    }

    /// Check for updates if it's been more than 24 hours since last check
    pub async fn check_for_updates(&mut self) -> Result<Option<ReleaseInfo>, Box<dyn std::error::Error + Send + Sync>> {
        let now = SystemTime::now();

        // Check if we should skip (less than 24 hours since last check)
        if let Some(last) = self.last_check {
            if now.duration_since(last).unwrap_or(Duration::from_secs(0)) < Duration::from_secs(24 * 60 * 60) {
                return Ok(None);
            }
        }

        self.last_check = Some(now);

        // Check GitHub releases
        let client = reqwest::Client::new();
        let url = "https://api.github.com/repos/rview-app/rview/releases/latest";

        let response = client
            .get(url)
            .header("User-Agent", "rView-UpdateChecker/1.0")
            .send()
            .await?;

        if !response.status().is_success() {
            return Ok(None);
        }

        let release: GitHubRelease = response.json().await?;

        // Compare versions (simple string comparison for now)
        if self.is_newer_version(&release.tag_name, &self.current_version) {
            Ok(Some(ReleaseInfo {
                tag_name: release.tag_name,
                name: release.name,
                body: release.body,
                published_at: release.published_at,
                html_url: release.html_url,
            }))
        } else {
            Ok(None)
        }
    }

    /// Simple version comparison (assumes semver format vX.Y.Z)
    fn is_newer_version(&self, remote: &str, local: &str) -> bool {
        // Remove 'v' prefix if present
        let remote_clean = remote.strip_prefix('v').unwrap_or(remote);
        let local_clean = local.strip_prefix('v').unwrap_or(local);

        // Simple string comparison - in production you'd want proper semver parsing
        remote_clean > local_clean
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        let checker = UpdateChecker::new("2.0.0".to_string());

        assert!(checker.is_newer_version("v2.1.0", "2.0.0"));
        assert!(checker.is_newer_version("2.1.0", "v2.0.0"));
        assert!(!checker.is_newer_version("v1.9.0", "2.0.0"));
        assert!(!checker.is_newer_version("2.0.0", "2.0.0"));
    }
}