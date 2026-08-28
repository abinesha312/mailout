//! LinkedIn profile scraping

use anyhow::{Context, Result, anyhow};
use scraper::{Html, Selector};
use std::collections::HashMap;

use crate::models::LinkedInProfile;
use crate::extractor::EmailExtractor;

pub struct LinkedInScraper {
    extractor: EmailExtractor,
}

impl LinkedInScraper {
    pub fn new() -> Self {
        LinkedInScraper {
            extractor: EmailExtractor::new(),
        }
    }

    pub fn normalize_url(input: &str) -> Result<(String, String)> {
        let input = input.trim();

        let slug = if input.starts_with("http://") || input.starts_with("https://") {
            if !input.contains("linkedin.com/in/") {
                return Err(anyhow!("Not a LinkedIn profile URL"));
            }
            input
                .split("linkedin.com/in/")
                .nth(1)
                .context("Invalid LinkedIn URL")?
                .trim_end_matches('/')
                .split('?')
                .next()
                .unwrap_or("")
                .to_string()
        } else {
            input.to_string()
        };

        if slug.is_empty() {
            return Err(anyhow!("Empty LinkedIn slug"));
        }

        let url = format!("https://www.linkedin.com/in/{}", slug);
        Ok((slug, url))
    }

    pub fn parse_profile(&self, html: &str, slug: String, url: String) -> Result<LinkedInProfile> {
        let document = Html::parse_document(html);

        let full_name = self.extract_name(&document);
        let headline = self.extract_headline(&document);
        let company = self.extract_company(&document);
        let location = self.extract_location(&document);
        let website = self.extract_website(&document);

        let company_domain = if let Some(ref w) = website {
            self.extract_domain_from_url(w)
        } else {
            None
        };

        Ok(LinkedInProfile {
            slug,
            url,
            full_name,
            headline,
            company,
            company_domain,
            location,
            website,
        })
    }

    pub fn extract_emails(&self, html: &str) -> Vec<String> {
        self.extractor.extract_from_html(html)
    }

    fn extract_name(&self, document: &Html) -> Option<String> {
        let selectors = vec![
            "h1.text-heading-xlarge",
            "h1",
            ".pv-text-details__left-panel h1",
            "[class*='top-card'] h1",
        ];

        for selector_str in selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                if let Some(element) = document.select(&selector).next() {
                    let text = element.text().collect::<String>().trim().to_string();
                    if !text.is_empty() && text.len() < 200 {
                        return Some(text);
                    }
                }
            }
        }

        None
    }

    fn extract_headline(&self, document: &Html) -> Option<String> {
        let selectors = vec![
            ".text-body-medium",
            ".pv-text-details__left-panel .text-body-medium",
            "[class*='top-card'] [class*='headline']",
        ];

        for selector_str in selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                if let Some(element) = document.select(&selector).next() {
                    let text = element.text().collect::<String>().trim().to_string();
                    if !text.is_empty() && text.len() < 500 {
                        return Some(text);
                    }
                }
            }
        }

        None
    }

    fn extract_company(&self, document: &Html) -> Option<String> {
        let text = document.root_element().text().collect::<String>();

        for line in text.lines() {
            let line = line.trim();
            if line.contains("Company Name") {
                continue;
            }
            if line.len() > 2 && line.len() < 100 {
                if line.contains(" at ") {
                    if let Some(company) = line.split(" at ").nth(1) {
                        return Some(company.trim().to_string());
                    }
                }
            }
        }

        None
    }

    fn extract_location(&self, document: &Html) -> Option<String> {
        let selectors = vec![
            ".text-body-small",
            ".pv-text-details__left-panel .text-body-small",
        ];

        for selector_str in selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                for element in document.select(&selector) {
                    let text = element.text().collect::<String>().trim().to_string();
                    if text.contains(',') || text.contains("Area") {
                        return Some(text);
                    }
                }
            }
        }

        None
    }

    fn extract_website(&self, document: &Html) -> Option<String> {
        if let Ok(selector) = Selector::parse("a[href]") {
            for element in document.select(&selector) {
                if let Some(href) = element.value().attr("href") {
                    if href.starts_with("http") && !href.contains("linkedin.com") {
                        return Some(href.to_string());
                    }
                }
            }
        }

        None
    }

    fn extract_domain_from_url(&self, url: &str) -> Option<String> {
        if let Ok(parsed) = url::Url::parse(url) {
            if let Some(host) = parsed.host_str() {
                return Some(host.to_string());
            }
        }
        None
    }
}

impl Default for LinkedInScraper {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn fetch_profile(url: &str, brightdata_token: Option<&str>) -> Result<String> {
    if let Some(token) = brightdata_token {
        fetch_with_brightdata(url, token).await
    } else {
        fetch_direct(url).await
    }
}

async fn fetch_direct(url: &str) -> Result<String> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let response = client
        .get(url)
        .send()
        .await
        .context("Failed to fetch LinkedIn profile")?;

    let status = response.status();
    let body = response.text().await?;

    if status == reqwest::StatusCode::UNAUTHORIZED || body.contains("authwall") {
        return Err(anyhow!(
            "LinkedIn requires login. Consider using BRIGHTDATA_API_TOKEN to bypass."
        ));
    }

    if !status.is_success() {
        return Err(anyhow!("HTTP {}: {}", status, body));
    }

    Ok(body)
}

async fn fetch_with_brightdata(url: &str, token: &str) -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()?;

    let mut params = HashMap::new();
    params.insert("url", url);

    let response = client
        .get("https://api.brightdata.com/serp")
        .header("Authorization", format!("Bearer {}", token))
        .query(&params)
        .send()
        .await
        .context("Failed to fetch via Bright Data")?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Bright Data returned status: {}",
            response.status()
        ));
    }

    let body = response.text().await?;
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_url() {
        let (slug, url) = LinkedInScraper::normalize_url("aharido").unwrap();
        assert_eq!(slug, "aharido");
        assert_eq!(url, "https://www.linkedin.com/in/aharido");

        let (slug, url) =
            LinkedInScraper::normalize_url("https://www.linkedin.com/in/aharido/").unwrap();
        assert_eq!(slug, "aharido");
        assert_eq!(url, "https://www.linkedin.com/in/aharido");
    }

    #[test]
    fn test_parse_profile_basic() {
        let scraper = LinkedInScraper::new();
        let html = r#"
            <html>
                <h1 class="text-heading-xlarge">John Doe</h1>
                <div class="text-body-medium">Software Engineer at Example Corp</div>
            </html>
        "#;

        let profile = scraper
            .parse_profile(html, "johndoe".to_string(), "https://www.linkedin.com/in/johndoe".to_string())
            .unwrap();

        assert_eq!(profile.slug, "johndoe");
        assert_eq!(profile.full_name, Some("John Doe".to_string()));
    }
}
