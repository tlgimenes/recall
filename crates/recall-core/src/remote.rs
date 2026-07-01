/// Normalize a git remote URL to a canonical `host/owner/repo` identifier so the
/// same repository maps to the same conventions regardless of clone URL form.
pub fn normalize_remote(url: &str) -> String {
    let mut u = url.trim().to_string();
    for p in ["https://", "http://", "ssh://", "git://"] {
        if let Some(rest) = u.strip_prefix(p) {
            u = rest.to_string();
        }
    }
    if let Some(rest) = u.strip_prefix("git@") {
        u = rest.to_string();
    }
    // Drop any remaining userinfo (e.g. token creds) before the host.
    if let Some(at) = u.rfind('@') {
        u = u[at + 1..].to_string();
    }
    // scp-style "host:owner/repo" → "host/owner/repo".
    u = u.replacen(':', "/", 1);
    if let Some(rest) = u.strip_suffix(".git") {
        u = rest.to_string();
    }
    u.trim_end_matches('/').to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::normalize_remote;

    #[test]
    fn https_with_token_creds() {
        assert_eq!(
            normalize_remote("https://x-access-token:ghs_ABC@github.com/Me/App.git"),
            "github.com/me/app"
        );
    }

    #[test]
    fn ssh_scp_form() {
        assert_eq!(
            normalize_remote("git@github.com:me/app.git"),
            "github.com/me/app"
        );
    }

    #[test]
    fn ssh_url_form() {
        assert_eq!(
            normalize_remote("ssh://git@github.com/me/app.git"),
            "github.com/me/app"
        );
    }

    #[test]
    fn plain_https_no_dot_git() {
        assert_eq!(
            normalize_remote("https://github.com/me/app"),
            "github.com/me/app"
        );
    }
}
