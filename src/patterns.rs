//! Email pattern generation

use crate::models::{EmailCandidate, EmailType, ConfidenceLevel};

pub fn generate_patterns(
    first_name: &str,
    last_name: &str,
    domain: &str,
) -> Vec<EmailCandidate> {
    let first = first_name.to_lowercase().trim().replace(' ', "");
    let last = last_name.to_lowercase().trim().replace(' ', "");
    let domain = domain.to_lowercase().trim().to_string();

    if first.is_empty() || last.is_empty() || domain.is_empty() {
        return vec![];
    }

    let first_initial = first.chars().next().unwrap_or('_');
    let last_initial = last.chars().next().unwrap_or('_');

    let patterns = vec![
        format!("{}.{}@{}", first, last, domain),
        format!("{}{}@{}", first, last, domain),
        format!("{}.{}@{}", first_initial, last, domain),
        format!("{}{}@{}", first, last_initial, domain),
        format!("{}@{}", first, domain),
        format!("{}.{}@{}", last, first, domain),
    ];

    patterns
        .into_iter()
        .filter(|p| !p.starts_with('.') && !p.contains(".."))
        .take(3)
        .map(|address| EmailCandidate {
            address,
            email_type: EmailType::Work,
            source: "pattern".to_string(),
            confidence: ConfidenceLevel::Guessed,
            verified: false,
            mx_valid: None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_patterns() {
        let patterns = generate_patterns("John", "Doe", "example.com");
        assert!(!patterns.is_empty());
        assert!(patterns.iter().any(|p| p.address == "john.doe@example.com"));
        assert!(patterns.iter().any(|p| p.address == "johndoe@example.com"));
    }

    #[test]
    fn test_generate_patterns_with_spaces() {
        let patterns = generate_patterns("Mary Jane", "Smith Wilson", "company.com");
        assert!(!patterns.is_empty());
        for pattern in &patterns {
            assert!(!pattern.address.contains(' '));
        }
    }

    #[test]
    fn test_generate_patterns_empty() {
        let patterns = generate_patterns("", "Doe", "example.com");
        assert!(patterns.is_empty());
    }
}
