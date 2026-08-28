//! MailOut - Find work emails from LinkedIn profiles using public web data

pub mod api;
pub mod cli;
pub mod db;
pub mod extractor;
pub mod lookup;
pub mod models;
pub mod patterns;
pub mod scraper;
pub mod search;

pub use db::Database;
pub use lookup::EmailLookup;
pub use models::{EmailCandidate, LinkedInProfile, LookupResult};
