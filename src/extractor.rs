//! Email extraction from HTML and text

use regex::Regex;
use scraper::{Html, Selector};
use std::collections::HashSet;

pub struct EmailExtractor {
    email_regex: Regex,
    personal_domains: HashSet<String>,
}

impl EmailExtractor {
    pub fn new() -> Self {
        let email_regex = Regex::new(
            r"(?i)\b[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}\b"
        ).unwrap();

        let personal_domains = vec![
            "gmail.com",
            "yahoo.com",
            "hotmail.com",
            "outlook.com",
            "icloud.com",
            "aol.com",
            "protonmail.com",
            "proton.me",
            "live.com",
            "msn.com",
            "mail.com",
            "yandex.com",
            "zoho.com",
            "gmx.com",
            "fastmail.com",
            "me.com",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        EmailExtractor {
            email_regex,
            personal_domains,
        }
    }

    pub fn extract_from_html(&self, html: &str) -> Vec<String> {
        let mut emails = HashSet::new();

        let document = Html::parse_document(html);

        if let Ok(mailto_selector) = Selector::parse("a[href^='mailto:']") {
            for element in document.select(&mailto_selector) {
                if let Some(href) = element.value().attr("href") {
                    if let Some(email) = href.strip_prefix("mailto:") {
                        let clean = email.split('?').next().unwrap_or(email);
                        if self.is_valid_email(clean) {
                            emails.insert(clean.to_lowercase());
                        }
                    }
                }
            }
        }

        let text = document.root_element().text().collect::<String>();
        for email in self.extract_from_text(&text) {
            emails.insert(email);
        }

        emails.into_iter().collect()
    }

    pub fn extract_from_text(&self, text: &str) -> Vec<String> {
        let mut emails = HashSet::new();

        for cap in self.email_regex.captures_iter(text) {
            if let Some(email) = cap.get(0) {
                let email_str = email.as_str().to_lowercase();
                if self.is_valid_email(&email_str) {
                    emails.insert(email_str);
                }
            }
        }

        emails.into_iter().collect()
    }

    pub fn is_personal_email(&self, email: &str) -> bool {
        if let Some(domain) = email.split('@').nth(1) {
            self.personal_domains.contains(domain)
        } else {
            false
        }
    }

    pub fn is_work_email(&self, email: &str) -> bool {
        !self.is_personal_email(email)
    }

    fn is_valid_email(&self, email: &str) -> bool {
        if email.len() > 254 {
            return false;
        }

        let parts: Vec<&str> = email.split('@').collect();
        if parts.len() != 2 {
            return false;
        }

        let local = parts[0];
        let domain = parts[1];

        if local.is_empty() || domain.is_empty() {
            return false;
        }

        if local.len() > 64 {
            return false;
        }

        if domain.starts_with('.') || domain.ends_with('.') {
            return false;
        }

        !domain.contains("..") && domain.contains('.')
    }
}

impl Default for EmailExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_from_text() {
        let extractor = EmailExtractor::new();
        let text = "Contact me at john.doe@example.com or jane@company.co.uk";
        let emails = extractor.extract_from_text(text);
        assert_eq!(emails.len(), 2);
        assert!(emails.contains(&"john.doe@example.com".to_string()));
        assert!(emails.contains(&"jane@company.co.uk".to_string()));
    }

    #[test]
    fn test_extract_from_html() {
        let extractor = EmailExtractor::new();
        let html = r#"<a href="mailto:contact@example.com">Email us</a>"#;
        let emails = extractor.extract_from_html(html);
        assert!(emails.contains(&"contact@example.com".to_string()));
    }

    #[test]
    fn test_is_personal_email() {
        let extractor = EmailExtractor::new();
        assert!(extractor.is_personal_email("user@gmail.com"));
        assert!(extractor.is_personal_email("user@yahoo.com"));
        assert!(!extractor.is_personal_email("user@company.com"));
    }

    #[test]
    fn test_is_work_email() {
        let extractor = EmailExtractor::new();
        assert!(extractor.is_work_email("user@company.com"));
        assert!(!extractor.is_work_email("user@gmail.com"));
    }
}
