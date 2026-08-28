//! Main lookup coordinator

use anyhow::Result;
use std::collections::HashMap;

use crate::db::Database;
use crate::models::{EmailCandidate, LinkedInProfile, LookupResult, EmailType, ConfidenceLevel};
use crate::scraper::{LinkedInScraper, fetch_profile};
use crate::search::WebSearcher;
use crate::patterns::generate_patterns;
use crate::extractor::EmailExtractor;

pub struct EmailLookup {
    db: Database,
    scraper: LinkedInScraper,
    searcher: WebSearcher,
    extractor: EmailExtractor,
    brightdata_token: Option<String>,
}

impl EmailLookup {
    pub fn new(db: Database, brightdata_token: Option<String>) -> Self {
        EmailLookup {
            db,
            scraper: LinkedInScraper::new(),
            searcher: WebSearcher::new(),
            extractor: EmailExtractor::new(),
            brightdata_token,
        }
    }

    pub async fn lookup(&self, linkedin_input: &str) -> Result<LookupResult> {
        let (slug, url) = LinkedInScraper::normalize_url(linkedin_input)?;

        self.db.record_lookup(&url, None)?;

        if let Some(cached) = self.get_cached_result(&slug)? {
            return Ok(cached);
        }

        let mut errors = Vec::new();
        let profile = self.fetch_and_parse_profile(&slug, &url, &mut errors).await;

        let mut all_emails = Vec::new();

        if let Some(ref profile_data) = profile {
            all_emails.extend(self.search_web_for_emails(profile_data, &mut errors).await);

            all_emails.extend(self.generate_pattern_emails(profile_data));

            let person_id = self.db.upsert_person(profile_data)?;
            self.db.record_lookup(&url, Some(person_id))?;

            for email in &all_emails {
                let _ = self.db.add_email(person_id, email);
            }
        }

        all_emails = self.deduplicate_emails(all_emails);

        Ok(LookupResult {
            profile: profile.unwrap_or_else(|| LinkedInProfile {
                slug: slug.clone(),
                url: url.clone(),
                full_name: None,
                headline: None,
                company: None,
                company_domain: None,
                location: None,
                website: None,
            }),
            emails: all_emails,
            errors,
            cached: false,
        })
    }

    async fn fetch_and_parse_profile(
        &self,
        slug: &str,
        url: &str,
        errors: &mut Vec<String>,
    ) -> Option<LinkedInProfile> {
        match fetch_profile(url, self.brightdata_token.as_deref()).await {
            Ok(html) => {
                self.db.record_page(url, 200, None).ok();

                let profile_emails = self.extractor.extract_from_html(&html);

                let profile = self
                    .scraper
                    .parse_profile(&html, slug.to_string(), url.to_string())
                    .ok()?;

                if !profile_emails.is_empty() {
                    tracing::info!("Found {} emails on LinkedIn profile page", profile_emails.len());
                }

                Some(profile)
            }
            Err(e) => {
                let error_msg = format!("Failed to fetch LinkedIn profile: {}", e);
                tracing::warn!("{}", error_msg);
                errors.push(error_msg);
                self.db.record_page(url, 0, Some(&e.to_string())).ok();
                None
            }
        }
    }

    async fn search_web_for_emails(
        &self,
        profile: &LinkedInProfile,
        _errors: &mut Vec<String>,
    ) -> Vec<EmailCandidate> {
        if let Some(ref name) = profile.full_name {
            match self
                .searcher
                .search_for_emails(name, profile.company_domain.as_deref())
                .await
            {
                emails => {
                    tracing::info!("Found {} emails from web search", emails.len());
                    return emails;
                }
            }
        }

        vec![]
    }

    fn generate_pattern_emails(&self, profile: &LinkedInProfile) -> Vec<EmailCandidate> {
        if let (Some(ref full_name), Some(ref domain)) = (&profile.full_name, &profile.company_domain)
        {
            let parts: Vec<&str> = full_name.split_whitespace().collect();
            if parts.len() >= 2 {
                let first = parts[0];
                let last = parts[parts.len() - 1];
                let patterns = generate_patterns(first, last, domain);
                tracing::info!("Generated {} pattern emails", patterns.len());
                return patterns;
            }
        }

        vec![]
    }

    fn deduplicate_emails(&self, mut emails: Vec<EmailCandidate>) -> Vec<EmailCandidate> {
        let mut seen = HashMap::new();

        emails.retain(|email| {
            let addr = email.address.to_lowercase();
            if seen.contains_key(&addr) {
                return false;
            }
            seen.insert(addr, true);
            true
        });

        emails.sort_by(|a, b| {
            let type_order = match (&a.email_type, &b.email_type) {
                (EmailType::Work, EmailType::Personal) => std::cmp::Ordering::Less,
                (EmailType::Personal, EmailType::Work) => std::cmp::Ordering::Greater,
                _ => std::cmp::Ordering::Equal,
            };

            if type_order != std::cmp::Ordering::Equal {
                return type_order;
            }

            let conf_order = vec![
                ConfidenceLevel::Verified,
                ConfidenceLevel::High,
                ConfidenceLevel::Medium,
                ConfidenceLevel::Low,
                ConfidenceLevel::Guessed,
            ];

            let a_idx = conf_order.iter().position(|c| c == &a.confidence).unwrap_or(99);
            let b_idx = conf_order.iter().position(|c| c == &b.confidence).unwrap_or(99);

            a_idx.cmp(&b_idx)
        });

        emails
    }

    fn get_cached_result(&self, slug: &str) -> Result<Option<LookupResult>> {
        if let Some(person) = self.db.get_person_by_slug(slug)? {
            let now = chrono::Utc::now();
            let age = now.signed_duration_since(person.fetched_at);

            if age.num_hours() < 24 {
                let profile: LinkedInProfile = serde_json::from_str(&person.raw_json)?;
                let db_emails = self.db.get_emails_for_person(person.id)?;

                let emails: Vec<EmailCandidate> = db_emails
                    .into_iter()
                    .map(|e| {
                        let email_type = match e.kind.as_str() {
                            "work" => EmailType::Work,
                            "personal" => EmailType::Personal,
                            _ => EmailType::Unknown,
                        };

                        let confidence = match e.confidence.as_str() {
                            "verified" => ConfidenceLevel::Verified,
                            "high" => ConfidenceLevel::High,
                            "medium" => ConfidenceLevel::Medium,
                            "low" => ConfidenceLevel::Low,
                            "guessed" => ConfidenceLevel::Guessed,
                            _ => ConfidenceLevel::Medium,
                        };

                        EmailCandidate {
                            address: e.address,
                            email_type,
                            source: e.source,
                            confidence,
                            verified: e.verified,
                            mx_valid: None,
                        }
                    })
                    .collect();

                return Ok(Some(LookupResult {
                    profile,
                    emails,
                    errors: vec![],
                    cached: true,
                }));
            }
        }

        Ok(None)
    }
}
