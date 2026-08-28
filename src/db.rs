//! SQLite database operations

use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, params};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use chrono::{DateTime, Utc};

use crate::models::{Person, Email, EmailCandidate, LinkedInProfile};

#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn new(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path)
            .context("Failed to open database")?;

        let db = Database { 
            conn: Arc::new(Mutex::new(conn))
        };
        db.init_schema()?;
        Ok(db)
    }

    pub fn default_path() -> Result<PathBuf> {
        if let Ok(db_path) = std::env::var("MAILOUT_DB") {
            return Ok(PathBuf::from(db_path));
        }

        let home = home::home_dir()
            .context("Could not determine home directory")?;
        Ok(home.join(".mailout").join("mailout.db"))
    }

    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS people (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                linkedin_slug TEXT UNIQUE NOT NULL,
                full_name TEXT,
                headline TEXT,
                company TEXT,
                company_domain TEXT,
                location TEXT,
                website TEXT,
                raw_json TEXT NOT NULL,
                fetched_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS emails (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                person_id INTEGER NOT NULL,
                address TEXT NOT NULL,
                kind TEXT NOT NULL,
                source TEXT NOT NULL,
                confidence TEXT NOT NULL,
                verified INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY (person_id) REFERENCES people(id),
                UNIQUE(person_id, address)
            );

            CREATE TABLE IF NOT EXISTS pages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                url TEXT UNIQUE NOT NULL,
                status INTEGER NOT NULL,
                excerpt TEXT,
                fetched_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS lookups (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                query TEXT NOT NULL,
                person_id INTEGER,
                created_at TEXT NOT NULL,
                FOREIGN KEY (person_id) REFERENCES people(id)
            );

            CREATE INDEX IF NOT EXISTS idx_emails_person ON emails(person_id);
            CREATE INDEX IF NOT EXISTS idx_lookups_created ON lookups(created_at);
            "#
        )?;
        Ok(())
    }

    pub fn upsert_person(&self, profile: &LinkedInProfile) -> Result<i64> {
        let raw_json = serde_json::to_string(profile)?;
        let now = Utc::now().to_rfc3339();

        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            INSERT INTO people (linkedin_slug, full_name, headline, company, company_domain, location, website, raw_json, fetched_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ON CONFLICT(linkedin_slug) DO UPDATE SET
                full_name = excluded.full_name,
                headline = excluded.headline,
                company = excluded.company,
                company_domain = excluded.company_domain,
                location = excluded.location,
                website = excluded.website,
                raw_json = excluded.raw_json,
                fetched_at = excluded.fetched_at
            "#,
            params![
                profile.slug,
                profile.full_name,
                profile.headline,
                profile.company,
                profile.company_domain,
                profile.location,
                profile.website,
                raw_json,
                now,
            ],
        )?;

        let person_id = conn.last_insert_rowid();
        Ok(person_id)
    }

    pub fn get_person_by_slug(&self, slug: &str) -> Result<Option<Person>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, linkedin_slug, full_name, headline, company, company_domain, location, website, raw_json, fetched_at FROM people WHERE linkedin_slug = ?1"
        )?;

        let person = stmt.query_row(params![slug], |row| {
            let fetched_at_str: String = row.get(9)?;
            let fetched_at = DateTime::parse_from_rfc3339(&fetched_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            
            Ok(Person {
                id: row.get(0)?,
                linkedin_slug: row.get(1)?,
                full_name: row.get(2)?,
                headline: row.get(3)?,
                company: row.get(4)?,
                company_domain: row.get(5)?,
                location: row.get(6)?,
                website: row.get(7)?,
                raw_json: row.get(8)?,
                fetched_at,
            })
        }).optional()?;

        Ok(person)
    }

    pub fn add_email(&self, person_id: i64, email: &EmailCandidate) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            INSERT INTO emails (person_id, address, kind, source, confidence, verified)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(person_id, address) DO UPDATE SET
                kind = excluded.kind,
                source = excluded.source,
                confidence = excluded.confidence,
                verified = excluded.verified
            "#,
            params![
                person_id,
                email.address,
                format!("{:?}", email.email_type).to_lowercase(),
                email.source,
                format!("{:?}", email.confidence).to_lowercase(),
                email.verified as i32,
            ],
        )?;
        Ok(())
    }

    pub fn get_emails_for_person(&self, person_id: i64) -> Result<Vec<Email>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, person_id, address, kind, source, confidence, verified FROM emails WHERE person_id = ?1"
        )?;

        let emails = stmt.query_map(params![person_id], |row| {
            Ok(Email {
                id: row.get(0)?,
                person_id: row.get(1)?,
                address: row.get(2)?,
                kind: row.get(3)?,
                source: row.get(4)?,
                confidence: row.get(5)?,
                verified: row.get::<_, i32>(6)? != 0,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

        Ok(emails)
    }

    pub fn record_lookup(&self, query: &str, person_id: Option<i64>) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO lookups (query, person_id, created_at) VALUES (?1, ?2, ?3)",
            params![query, person_id, now],
        )?;
        Ok(())
    }

    pub fn record_page(&self, url: &str, status: u16, excerpt: Option<&str>) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            INSERT INTO pages (url, status, excerpt, fetched_at)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(url) DO UPDATE SET
                status = excluded.status,
                excerpt = excluded.excerpt,
                fetched_at = excluded.fetched_at
            "#,
            params![url, status as i32, excerpt, now],
        )?;
        Ok(())
    }

    pub fn stats(&self) -> Result<DatabaseStats> {
        let conn = self.conn.lock().unwrap();
        let people_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM people",
            [],
            |row| row.get(0)
        )?;

        let emails_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM emails",
            [],
            |row| row.get(0)
        )?;

        let lookups_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM lookups",
            [],
            |row| row.get(0)
        )?;

        let pages_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM pages",
            [],
            |row| row.get(0)
        )?;

        Ok(DatabaseStats {
            people: people_count,
            emails: emails_count,
            lookups: lookups_count,
            pages: pages_count,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct DatabaseStats {
    pub people: i64,
    pub emails: i64,
    pub lookups: i64,
    pub pages: i64,
}

use serde::Serialize;
