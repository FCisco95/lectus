//! Pure validation at the subscription trust boundaries.
use anyhow::{bail, Result};
use serde_json::Value;

pub fn completed_text(data: &[u8]) -> Result<String> {
    if data.len() > 1024 * 1024 {
        bail!("ChatGPT response exceeded the size limit.");
    }
    let wire = std::str::from_utf8(data)
        .map_err(|_| anyhow::anyhow!("ChatGPT returned invalid text."))?
        .replace("\r\n", "\n");
    let mut completed = None;
    for frame in wire.split("\n\n") {
        let payload = frame
            .lines()
            .filter_map(|line| line.strip_prefix("data:").map(str::trim_start))
            .collect::<Vec<_>>()
            .join("\n");
        if payload.is_empty() || payload == "[DONE]" {
            continue;
        }
        let event: Value = serde_json::from_str(&payload)
            .map_err(|_| anyhow::anyhow!("ChatGPT returned an invalid event."))?;
        match event["type"].as_str().unwrap_or("") {
            "response.failed"
            | "response.incomplete"
            | "error"
            | "response.refusal.delta"
            | "response.refusal.done" => bail!(
                "ChatGPT could not complete the response. Check subscription limits and try again."
            ),
            "response.completed" => {
                if completed.is_some() || event["response"]["status"] != "completed" {
                    bail!("ChatGPT returned an invalid completion.");
                }
                let output = event["response"]["output"]
                    .as_array()
                    .ok_or_else(|| anyhow::anyhow!("ChatGPT returned no completed text."))?;
                let mut text = String::new();
                for item in output {
                    if item["type"] == "reasoning" {
                        continue;
                    }
                    if item["type"] != "message"
                        || item.get("status").is_some_and(|s| s != "completed")
                    {
                        bail!("ChatGPT returned unsupported output.");
                    }
                    for part in item["content"]
                        .as_array()
                        .ok_or_else(|| anyhow::anyhow!("ChatGPT returned invalid content."))?
                    {
                        if part["type"] != "output_text" {
                            bail!("ChatGPT refused or returned unsupported content.");
                        }
                        text.push_str(
                            part["text"]
                                .as_str()
                                .ok_or_else(|| anyhow::anyhow!("ChatGPT returned invalid text."))?,
                        );
                        if text.len() > 64 * 1024 {
                            bail!("ChatGPT text exceeded the size limit.");
                        }
                    }
                }
                if text.trim().is_empty() {
                    bail!("ChatGPT returned empty text.");
                }
                completed = Some(text);
            }
            _ => {}
        }
    }
    completed.ok_or_else(|| anyhow::anyhow!("ChatGPT stream ended before completion."))
}

pub fn claude_subscription(value: &Value) -> bool {
    value["loggedIn"] == true
        && value["authMethod"] == "claude.ai"
        && value.get("apiProvider").is_none_or(|p| p == "firstParty")
        && matches!(
            value["subscriptionType"].as_str(),
            Some("pro" | "max" | "team" | "enterprise")
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn complete_response_accepts_only_final_text() {
        let data = b"data: {\"type\":\"response.output_text.delta\",\"delta\":\"wrong\"}\n\ndata: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\",\"output\":[{\"type\":\"message\",\"status\":\"completed\",\"content\":[{\"type\":\"output_text\",\"text\":\"final\"}]}]}}\n\n";
        assert_eq!(completed_text(data).unwrap(), "final");
    }
    #[test]
    fn partial_and_late_failures_release_no_text() {
        for data in [
            "data: {\"type\":\"response.output_text.delta\",\"delta\":\"secret\"}\n\n",
            "data: {\"type\":\"response.incomplete\"}\n\n",
            "data: {\"type\":\"response.failed\",\"error\":\"secret\"}\n\n",
            "data: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\",\"output\":[{\"type\":\"message\",\"content\":[{\"type\":\"refusal\",\"refusal\":\"no\"}]}]}}\n\n",
        ] { assert!(completed_text(data.as_bytes()).is_err()); }
    }
    #[test]
    fn cli_existence_or_api_auth_is_not_subscription_auth() {
        assert!(claude_subscription(
            &serde_json::json!({"loggedIn":true,"authMethod":"claude.ai","apiProvider":"firstParty","subscriptionType":"max"})
        ));
        for method in [
            "none",
            "api_key",
            "api_key_helper",
            "third_party",
            "oauth_token",
        ] {
            assert!(!claude_subscription(
                &serde_json::json!({"loggedIn":true,"authMethod":method,"subscriptionType":"max"})
            ));
        }
        assert!(!claude_subscription(
            &serde_json::json!({"loggedIn":true,"authMethod":"claude.ai","subscriptionType":null})
        ));
    }
}
