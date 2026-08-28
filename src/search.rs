//! Web search for emails

use anyhow::{Context, Result};
use std::collections::HashSet;

use crate::extractor::EmailExtractor;
use crate::models::{EmailCandidate, EmailType, ConfidenceLevel};

pub struct WebSearcher {
    extractor: EmailExtractor,
    client: reqwest::Client,
}

impl WebSearcher {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .unwrap();

        WebSearcher {
            extractor: EmailExtractor::new(),
            client,
        }
    }

    pub async fn search_for_emails(
        &self,
        full_name: &str,
        company_domain: Option<&str>,
    ) -> Vec<EmailCandidate> {
        let mut all_emails = HashSet::new();

        let queries = self.build_search_queries(full_name, company_domain);

        for query in queries {
            if let Ok(emails) = self.search_duckduckgo(&query).await {
                all_emails.extend(emails);
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        if let Some(domain) = company_domain {
            if let Ok(emails) = self.fetch_company_pages(domain).await {
                all_emails.extend(emails);
            }
        }

        let mut candidates: Vec<EmailCandidate> = all_emails
            .into_iter()
            .map(|address| {
                let email_type = if self.extractor.is_work_email(&address) {
                    EmailType::Work
                } else {
                    EmailType::Personal
                };

                EmailCandidate {
                    address,
                    email_type,
                    source: "web_search".to_string(),
                    confidence: ConfidenceLevel::Medium,
                    verified: false,
                    mx_valid: None,
                }
            })
            .collect();

        candidates.sort_by(|a, b| {
            match (&a.email_type, &b.email_type) {
                (EmailType::Work, EmailType::Personal) => std::cmp::Ordering::Less,
                (EmailType::Personal, EmailType::Work) => std::cmp::Ordering::Greater,
                _ => std::cmp::Ordering::Equal,
            }
        });

        candidates
    }

    fn build_search_queries(&self, full_name: &str, company_domain: Option<&str>) -> Vec<String> {
        let mut queries = vec![format!("\"{}\" email", full_name)];

        if let Some(domain) = company_domain {
            queries.push(format!("\"{}\" @{}", full_name, domain));
            queries.push(format!("site:github.com \"{}\"", full_name));
        }

        queries
    }

    async fn search_duckduckgo(&self, query: &str) -> Result<Vec<String>> {
        let url = format!("https://html.duckduckgo.com/html/?q={}", urlencoding::encode(query));

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch DuckDuckGo results")?;

        if !response.status().is_success() {
            return Ok(vec![]);
        }

        let html = response.text().await?;
        let emails = self.extractor.extract_from_html(&html);

        Ok(emails)
    }

    async fn fetch_company_pages(&self, domain: &str) -> Result<Vec<String>> {
        let mut all_emails = Vec::new();

        let pages = vec![
            format!("https://{}/about", domain),
            format!("https://{}/contact", domain),
            format!("https://{}/team", domain),
            format!("https://www.{}/about", domain),
            format!("https://www.{}/contact", domain),
        ];

        for page_url in pages {
            if let Ok(response) = self.client.get(&page_url).send().await {
                if response.status().is_success() {
                    if let Ok(html) = response.text().await {
                        let emails = self.extractor.extract_from_html(&html);
                        all_emails.extend(emails);
                    }
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }

        Ok(all_emails)
    }
}

impl Default for WebSearcher {
    fn default() -> Self {
        Self::new()
    }
}

use urlencoding;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_search_queries() {
        let searcher = WebSearcher::new();
        let queries = searcher.build_search_queries("John Doe", Some("example.com"));

        assert!(queries.iter().any(|q| q.contains("John Doe")));
        assert!(queries.iter().any(|q| q.contains("@example.com")));
    }
}
