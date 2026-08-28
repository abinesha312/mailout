//! Data models for MailOut

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EmailType {
    Work,
    Personal,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConfidenceLevel {
    Verified,
    High,
    Medium,
    Low,
    Guessed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailCandidate {
    pub address: String,
    #[serde(rename = "type")]
    pub email_type: EmailType,
    pub source: String,
    pub confidence: ConfidenceLevel,
    pub verified: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mx_valid: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedInProfile {
    pub slug: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headline: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company_domain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LookupResult {
    pub profile: LinkedInProfile,
    pub emails: Vec<EmailCandidate>,
    pub errors: Vec<String>,
    pub cached: bool,
}

#[derive(Debug, Clone)]
pub struct Person {
    pub id: i64,
    pub linkedin_slug: String,
    pub full_name: Option<String>,
    pub headline: Option<String>,
    pub company: Option<String>,
    pub company_domain: Option<String>,
    pub location: Option<String>,
    pub website: Option<String>,
    pub raw_json: String,
    pub fetched_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct Email {
    pub id: i64,
    pub person_id: i64,
    pub address: String,
    pub kind: String,
    pub source: String,
    pub confidence: String,
    pub verified: bool,
}

#[derive(Debug, Clone)]
pub struct Page {
    pub id: i64,
    pub url: String,
    pub status: i32,
    pub excerpt: Option<String>,
    pub fetched_at: chrono::DateTime<chrono::Utc>,
}
