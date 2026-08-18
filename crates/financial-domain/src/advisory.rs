//! AI advisory text is not a posted fact (ADR-0012). Secrets never go into prompts.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvisoryRequest {
    pub prompt: String,
}

pub fn prepare_advisory(prompt: String) -> AdvisoryRequest {
    AdvisoryRequest {
        prompt: redact_for_advisory(&prompt),
    }
}

/// Strip API keys and bearer tokens before any prompt or stored run text.
pub fn redact_for_advisory(input: &str) -> String {
    let mut out = input.to_string();
    replace_prefixed(&mut out, "XAI_API_KEY=");
    replace_prefixed(&mut out, "GROK_API_KEY=");
    replace_prefixed(&mut out, "Bearer ");
    while let Some(start) = out.find("xai-") {
        let rest = &out[start + 4..];
        let take = rest
            .find(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_')
            .unwrap_or(rest.len());
        out.replace_range(start..start + 4 + take, "[redacted]");
    }
    out
}

fn replace_prefixed(text: &mut String, prefix: &str) {
    let mut from = 0;
    while let Some(rel) = text[from..].find(prefix) {
        let start = from + rel + prefix.len();
        let rest = &text[start..];
        let take = rest.find(char::is_whitespace).unwrap_or(rest.len());
        text.replace_range(start..start + take, "[redacted]");
        from = start + "[redacted]".len();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advisory_is_not_a_posted_activity() {
        let posted_before = 0i64;
        let req = prepare_advisory("Should I rebalance VXUS?".into());
        assert!(req.prompt.contains("VXUS"));
        assert_eq!(posted_before, 0);
    }

    #[test]
    fn secrets_are_stripped_from_prompts() {
        let redacted = redact_for_advisory("key XAI_API_KEY=secret-value and xai-abc123 end");
        assert!(!redacted.contains("secret-value"));
        assert!(!redacted.contains("xai-abc123"));
        assert!(redacted.contains("[redacted]"));
    }
}
