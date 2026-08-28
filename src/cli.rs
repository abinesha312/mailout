//! CLI interface

use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::api;
use crate::db::Database;
use crate::lookup::EmailLookup;

#[derive(Parser)]
#[command(name = "mailout")]
#[command(about = "Find work emails from LinkedIn profiles", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Look up emails for a LinkedIn profile")]
    Lookup {
        #[arg(help = "LinkedIn URL or username (e.g., aharido or https://linkedin.com/in/aharido)")]
        linkedin: String,

        #[arg(long, help = "Output as JSON")]
        json: bool,
    },

    #[command(about = "Start HTTP API server")]
    Serve {
        #[arg(long, default_value = "3000", help = "Port to listen on")]
        port: u16,
    },

    #[command(about = "Show database statistics")]
    Db {
        #[command(subcommand)]
        command: Option<DbCommands>,
    },
}

#[derive(Subcommand)]
pub enum DbCommands {
    #[command(about = "Show database statistics")]
    Stats,
}

pub async fn run(cli: Cli) -> Result<()> {
    let db_path = Database::default_path()?;
    let db = Database::new(&db_path)?;

    let brightdata_token = std::env::var("BRIGHTDATA_API_TOKEN").ok();

    match cli.command {
        Commands::Lookup { linkedin, json } => {
            let lookup = EmailLookup::new(db, brightdata_token);
            let result = lookup.lookup(&linkedin).await?;

            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                print_result(&result);
            }
        }

        Commands::Serve { port } => {
            api::serve(db, brightdata_token, port).await?;
        }

        Commands::Db { command } => {
            match command {
                Some(DbCommands::Stats) | None => {
                    let stats = db.stats()?;
                    println!("Database: {}", db_path.display());
                    println!("People:   {}", stats.people);
                    println!("Emails:   {}", stats.emails);
                    println!("Lookups:  {}", stats.lookups);
                    println!("Pages:    {}", stats.pages);
                }
            }
        }
    }

    Ok(())
}

fn print_result(result: &crate::models::LookupResult) {
    println!("\n{}", "=".repeat(60));
    println!("LinkedIn Profile");
    println!("{}", "=".repeat(60));

    let profile = &result.profile;
    println!("URL:      {}", profile.url);

    if let Some(ref name) = profile.full_name {
        println!("Name:     {}", name);
    }
    if let Some(ref headline) = profile.headline {
        println!("Headline: {}", headline);
    }
    if let Some(ref company) = profile.company {
        println!("Company:  {}", company);
    }
    if let Some(ref domain) = profile.company_domain {
        println!("Domain:   {}", domain);
    }
    if let Some(ref location) = profile.location {
        println!("Location: {}", location);
    }

    if result.cached {
        println!("\n(Cached result from database)");
    }

    println!("\n{}", "=".repeat(60));
    println!("Email Candidates ({})", result.emails.len());
    println!("{}", "=".repeat(60));

    if result.emails.is_empty() {
        println!("No emails found.");
        println!("\nNote: Public LinkedIn profiles rarely show email addresses.");
        println!("Consider:");
        println!("  - Using BRIGHTDATA_API_TOKEN to bypass login walls");
        println!("  - Checking if company domain is available for pattern generation");
    } else {
        for (i, email) in result.emails.iter().enumerate() {
            println!("\n{}. {}", i + 1, email.address);
            println!("   Type:       {:?}", email.email_type);
            println!("   Source:     {}", email.source);
            println!("   Confidence: {:?}", email.confidence);
            println!("   Verified:   {}", email.verified);
        }
    }

    if !result.errors.is_empty() {
        println!("\n{}", "=".repeat(60));
        println!("Errors");
        println!("{}", "=".repeat(60));
        for error in &result.errors {
            println!("  - {}", error);
        }
    }

    println!();
}
