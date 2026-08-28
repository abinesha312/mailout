use mailout::{Database, EmailLookup};
use mailout::models::{LinkedInProfile, EmailCandidate, EmailType, ConfidenceLevel};
use tempfile::TempDir;

#[tokio::test]
async fn test_database_operations() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let db = Database::new(&db_path).unwrap();

    let profile = LinkedInProfile {
        slug: "testuser".to_string(),
        url: "https://www.linkedin.com/in/testuser".to_string(),
        full_name: Some("Test User".to_string()),
        headline: Some("Software Engineer".to_string()),
        company: Some("Test Corp".to_string()),
        company_domain: Some("testcorp.com".to_string()),
        location: Some("San Francisco, CA".to_string()),
        website: None,
    };

    let person_id = db.upsert_person(&profile).unwrap();
    assert!(person_id > 0);

    let email = EmailCandidate {
        address: "test@testcorp.com".to_string(),
        email_type: EmailType::Work,
        source: "test".to_string(),
        confidence: ConfidenceLevel::High,
        verified: false,
        mx_valid: None,
    };

    db.add_email(person_id, &email).unwrap();

    let retrieved = db.get_person_by_slug("testuser").unwrap();
    assert!(retrieved.is_some());
    let person = retrieved.unwrap();
    assert_eq!(person.full_name, Some("Test User".to_string()));

    let emails = db.get_emails_for_person(person_id).unwrap();
    assert_eq!(emails.len(), 1);
    assert_eq!(emails[0].address, "test@testcorp.com");

    db.record_lookup("https://www.linkedin.com/in/testuser", Some(person_id))
        .unwrap();

    let stats = db.stats().unwrap();
    assert_eq!(stats.people, 1);
    assert_eq!(stats.emails, 1);
    assert_eq!(stats.lookups, 1);
}

#[tokio::test]
async fn test_email_lookup_with_cache() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let db = Database::new(&db_path).unwrap();

    let profile = LinkedInProfile {
        slug: "cacheduser".to_string(),
        url: "https://www.linkedin.com/in/cacheduser".to_string(),
        full_name: Some("Cached User".to_string()),
        headline: None,
        company: None,
        company_domain: None,
        location: None,
        website: None,
    };

    let person_id = db.upsert_person(&profile).unwrap();

    let email = EmailCandidate {
        address: "cached@example.com".to_string(),
        email_type: EmailType::Work,
        source: "test".to_string(),
        confidence: ConfidenceLevel::Medium,
        verified: false,
        mx_valid: None,
    };

    db.add_email(person_id, &email).unwrap();

    let lookup = EmailLookup::new(db, None);
    let result = lookup.lookup("cacheduser").await.unwrap();

    assert!(result.cached);
    assert_eq!(result.emails.len(), 1);
    assert_eq!(result.emails[0].address, "cached@example.com");
}

#[test]
fn test_normalize_linkedin_url() {
    use mailout::scraper::LinkedInScraper;

    let (slug, url) = LinkedInScraper::normalize_url("aharido").unwrap();
    assert_eq!(slug, "aharido");
    assert_eq!(url, "https://www.linkedin.com/in/aharido");

    let (slug, url) =
        LinkedInScraper::normalize_url("https://www.linkedin.com/in/johndoe/").unwrap();
    assert_eq!(slug, "johndoe");
    assert_eq!(url, "https://www.linkedin.com/in/johndoe");

    let (slug, url) = LinkedInScraper::normalize_url(
        "https://www.linkedin.com/in/janedoe?trk=profile",
    )
    .unwrap();
    assert_eq!(slug, "janedoe");
    assert_eq!(url, "https://www.linkedin.com/in/janedoe");
}

#[test]
fn test_pattern_generation() {
    use mailout::patterns::generate_patterns;

    let patterns = generate_patterns("John", "Doe", "example.com");
    assert!(!patterns.is_empty());
    assert!(patterns.iter().any(|p| p.address == "john.doe@example.com"));
    assert!(patterns.iter().any(|p| p.address == "johndoe@example.com"));

    for pattern in &patterns {
        assert_eq!(pattern.email_type, EmailType::Work);
        assert_eq!(pattern.confidence, ConfidenceLevel::Guessed);
        assert!(!pattern.verified);
    }
}

#[test]
fn test_email_extraction() {
    use mailout::extractor::EmailExtractor;

    let extractor = EmailExtractor::new();

    let html = r#"
        <html>
            <body>
                <a href="mailto:contact@example.com">Email us</a>
                <p>Or reach out to support@example.com</p>
            </body>
        </html>
    "#;

    let emails = extractor.extract_from_html(html);
    assert!(emails.contains(&"contact@example.com".to_string()));
    assert!(emails.contains(&"support@example.com".to_string()));
}

#[test]
fn test_work_vs_personal_email() {
    use mailout::extractor::EmailExtractor;

    let extractor = EmailExtractor::new();

    assert!(extractor.is_personal_email("user@gmail.com"));
    assert!(extractor.is_personal_email("user@yahoo.com"));
    assert!(extractor.is_personal_email("user@hotmail.com"));

    assert!(extractor.is_work_email("user@company.com"));
    assert!(extractor.is_work_email("user@startup.io"));
}
