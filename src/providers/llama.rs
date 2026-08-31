use std::time::Duration;

/// Normalize a provider base URL into the OpenAI-style `/v1/models` endpoint.
/// Handles the common spellings: `http://host:port`,
/// `http://host:port/`, `http://host:port/v1`, `http://host:port/v1/`.
fn models_url(base_url: &str) -> String {
    let base = base_url.trim().trim_end_matches('/');
    if base.ends_with("/v1") {
        format!("{base}/models")
    } else {
        format!("{base}/v1/models")
    }
}

/// Fetch the first model id from an OpenAI-compatible `/v1/models` endpoint
/// (e.g. llama.cpp's server). Returns a human-readable error on any failure.
pub fn detect_model(base_url: &str, auth_token: &str) -> Result<String, String> {
    let url = models_url(base_url);

    // End-to-end timeout covering DNS through reading the response body.
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(5))).build().into();

    let mut req = agent.get(&url).header("Accept", "application/json");
    if !auth_token.trim().is_empty() {
        req = req.header("Authorization", &format!("Bearer {}", auth_token.trim()));
    }

    let mut resp = req.call().map_err(|e| format!("Could not reach {url}: {e}"))?;

    let body: serde_json::Value = resp.body_mut().read_json().map_err(|e| format!("Invalid response from {url}: {e}"))?;

    let id = body
        .get("data")
        .and_then(|d| d.as_array())
        .and_then(|arr| arr.first())
        .and_then(|m| m.get("id"))
        .and_then(|i| i.as_str())
        .ok_or_else(|| format!("No model id found in {url} (server returned an empty model list)"))?;

    Ok(id.to_string())
}
