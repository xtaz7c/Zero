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
    let tag = v
        .get("tag_name")
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .trim_start_matches('v')
        .to_string();
    let html_url = v
        .get("html_url")
        .and_then(|u| u.as_str())
        .map(|s| s.to_string());

    if tag.is_empty() {
        return Err("resposta sem tag_name".into());
    }
    Ok((tag, html_url))
}

pub fn version_newer(remote: &str, local: &str) -> bool {
    let parse = |s: &str| -> Vec<u32> {
        s.split(|c: char| !c.is_ascii_digit())
            .filter(|p| !p.is_empty())
            .filter_map(|p| p.parse().ok())
            .collect()
    };
    let r = parse(remote);
    let l = parse(local);
    for i in 0..r.len().max(l.len()) {
        let rv = r.get(i).copied().unwrap_or(0);
        let lv = l.get(i).copied().unwrap_or(0);
        if rv > lv {
            return true;
        }
        if rv < lv {
            return false;
        }
    }
    false
}
