use std::time::Duration;

pub const APP_VERSION: &str = "1.0.0";
pub const GITHUB_RELEASES_URL: &str = "https://api.github.com/repos/xtaz7c/Zero/releases/latest";

pub fn check_github_update() -> Result<(String, Option<String>), String> {
    let resp = ureq::get(GITHUB_RELEASES_URL)
        .set("User-Agent", "Zero-App/1.0")
        .set("Accept", "application/vnd.github+json")
        .timeout(Duration::from_secs(8))
        .call()
        .map_err(|e| e.to_string())?;
    let v: serde_json::Value = resp.into_json().map_err(|e| e.to_string())?;
    let tag = v.get("tag_name").and_then(|t| t.as_str()).unwrap_or("").trim_start_matches('v').to_string();
    let html_url = v.get("html_url").and_then(|u| u.as_str()).map(str::to_string);
    if tag.is_empty() { return Err("resposta sem tag_name".into()); }
    Ok((tag, html_url))
}

pub fn version_newer(remote: &str, local: &str) -> bool {
    fn parse(s: &str) -> Option<Vec<u32>> {
        let parts: Vec<u32> = s.trim().trim_start_matches('v').split('.').map(str::parse).collect::<Result<_, _>>().ok()?;
        if parts.is_empty() { return None; }
        Some(parts)
    }
    let (Some(r), Some(l)) = (parse(remote), parse(local)) else { return false; };
    for i in 0..r.len().max(l.len()) {
        match (r.get(i).copied().unwrap_or(0), l.get(i).copied().unwrap_or(0)) {
            (a,b) if a>b => return true,
            (a,b) if a<b => return false,
            _ => {}
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::version_newer;
    #[test] fn newer_cases() { assert!(version_newer("v1.0.1","1.0.0")); assert!(version_newer("2.0.0","1.9.9")); assert!(version_newer("1.10.0","1.9.0")); }
    #[test] fn equal_or_older_cases() { assert!(!version_newer("1.0.0","1.0.0")); assert!(!version_newer("1.0.0","1.0.1")); assert!(!version_newer("v1.2","1.2.0")); }
    #[test] fn invalid_cases() { assert!(!version_newer("","1.0.0")); assert!(!version_newer("latest","1.0.0")); assert!(!version_newer("1.0.0","")); }
}
