//! Secret-pattern scanning for Write and Edit tool content.

/// Scan content for secret-like patterns.
/// Returns a list of human-readable descriptions of what was found.
pub fn scan_for_secrets(content: &str) -> Vec<String> {
    let mut findings = Vec::new();

    for (i, line) in content.lines().enumerate() {
        let lineno = i + 1;

        // API key patterns
        if has_pattern(line, "sk-ant-") {
            findings.push(format!(
                "line {}: possible Anthropic API key (sk-ant-...)",
                lineno
            ));
        } else if has_pattern(line, "sk-or-") {
            findings.push(format!(
                "line {}: possible OpenRouter API key (sk-or-...)",
                lineno
            ));
        } else if contains_openai_key(line) {
            findings.push(format!("line {}: possible OpenAI API key (sk-...)", lineno));
        }

        // AWS access key
        if contains_aws_key(line) {
            findings.push(format!(
                "line {}: possible AWS access key (AKIA...)",
                lineno
            ));
        }

        // Private key header
        if line.contains("-----BEGIN") && line.contains("PRIVATE KEY-----") {
            findings.push(format!(
                "line {}: possible private key (-----BEGIN ... PRIVATE KEY-----)",
                lineno
            ));
        }

        // Generic high-entropy tokens: _key, _secret, _token, _password followed by = and value >=16 chars
        if let Some(value) = extract_assignment_value(line) {
            let lower = line.to_lowercase();
            if (lower.contains("_key")
                || lower.contains("_secret")
                || lower.contains("_token")
                || lower.contains("_password"))
                && value.len() >= 16
            {
                findings.push(format!(
                    "line {}: possible credential assignment (key/secret/token/password)",
                    lineno
                ));
            }
        }
    }

    findings
}

fn has_pattern(line: &str, pattern: &str) -> bool {
    line.contains(pattern)
}

/// Check for OpenAI-style key: sk- followed by 20+ alphanumerics (but not sk-ant or sk-or)
fn contains_openai_key(line: &str) -> bool {
    let mut pos = 0;
    while let Some(idx) = line[pos..].find("sk-") {
        let abs = pos + idx;
        let rest = &line[abs + 3..];
        // skip sk-ant- and sk-or-
        if rest.starts_with("ant-") || rest.starts_with("or-") {
            pos = abs + 3;
            continue;
        }
        // Count alphanumeric chars
        let count = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '-')
            .count();
        if count >= 20 {
            return true;
        }
        pos = abs + 3;
    }
    false
}

/// Check for AWS access key: AKIA followed by 16 uppercase alphanumerics
fn contains_aws_key(line: &str) -> bool {
    let mut pos = 0;
    while let Some(idx) = line[pos..].find("AKIA") {
        let abs = pos + idx;
        let rest = &line[abs + 4..];
        let count = rest
            .chars()
            .take_while(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
            .count();
        if count >= 16 {
            return true;
        }
        pos = abs + 4;
    }
    false
}

/// Extract the value part of a `key = value` or `KEY=value` assignment.
/// Returns the trimmed value string if the line looks like an assignment.
fn extract_assignment_value(line: &str) -> Option<&str> {
    // Try `=` as separator
    let eq_pos = line.find('=')?;
    let value = line[eq_pos + 1..].trim();
    // Strip surrounding quotes
    let value = value.trim_matches('"').trim_matches('\'').trim_matches('`');
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_anthropic_key() {
        let content = "api_key = \"sk-ant-api03-abcdefghijklmnopqrstuvwxyz1234567890\"";
        let findings = scan_for_secrets(content);
        assert!(!findings.is_empty(), "Should detect Anthropic API key");
        assert!(findings[0].contains("Anthropic"));
    }

    #[test]
    fn detects_openrouter_key() {
        let content = "key = \"sk-or-v1-abcdefghijklmnopqrstuvwxyz1234567890abcdef\"";
        let findings = scan_for_secrets(content);
        assert!(!findings.is_empty(), "Should detect OpenRouter key");
        assert!(findings[0].contains("OpenRouter"));
    }

    #[test]
    fn detects_aws_key() {
        let content = "aws_key = \"AKIAIOSFODNN7EXAMPLE\"";
        let findings = scan_for_secrets(content);
        assert!(
            findings.iter().any(|f| f.contains("AWS")),
            "Should detect AWS key"
        );
    }

    #[test]
    fn detects_private_key_header() {
        let content =
            "-----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAKCAQEA...\n-----END RSA PRIVATE KEY-----";
        let findings = scan_for_secrets(content);
        assert!(
            findings.iter().any(|f| f.contains("private key")),
            "Should detect private key header"
        );
    }

    #[test]
    fn detects_generic_token() {
        let content = "auth_token = \"supersecretlongvalue1234567890\"";
        let findings = scan_for_secrets(content);
        assert!(
            findings.iter().any(|f| f.contains("credential")),
            "Should detect generic token assignment"
        );
    }

    #[test]
    fn no_false_positive_short_values() {
        let content = "api_key = \"short\"";
        let findings = scan_for_secrets(content);
        // Short value should not trigger generic token check
        assert!(
            !findings.iter().any(|f| f.contains("credential")),
            "Should not flag short values"
        );
    }

    #[test]
    fn clean_content_no_findings() {
        let content = "fn main() {\n    println!(\"hello world\");\n}\n";
        let findings = scan_for_secrets(content);
        assert!(findings.is_empty(), "Clean content should have no findings");
    }
}
