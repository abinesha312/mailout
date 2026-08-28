# MailOut

Find work emails from LinkedIn profiles using public web data.

**Note:** This is a B2B contact enrichment tool for professional outreach, not a spam/scraping/data-breach tool. Public LinkedIn profiles **rarely show email addresses** directly. This tool combines multiple public sources and pattern generation to find candidate emails.

## What It Does

Given a LinkedIn profile URL or username (e.g., `aharido` or `https://www.linkedin.com/in/aharido`):

1. **Fetches the public LinkedIn profile** (with optional Bright Data Web Unlocker to bypass login walls)
2. **Extracts profile data:** name, headline, company, location, website, and any emails visible on the page
3. **Searches the public web** for emails associated with the person's name and company
4. **Generates common email patterns** (e.g., `first.last@company.com`) when company domain is known
5. **Stores everything in a local SQLite database** for caching and analysis
6. **Returns JSON** with profile data and candidate emails ranked by confidence

## What It Does NOT Do

- Does NOT scrape email dump sites or data breaches
- Does NOT use third-party paid enrichment APIs (Hunter, Apollo, ZoomInfo, etc.)
- Does NOT harvest personal Gmail/Yahoo addresses by default (filters them as "personal")
- Does NOT bulk scrape LinkedIn (one profile at a time, with caching)
- Does NOT bypass LinkedIn's Terms of Service beyond fetching public profile pages

## Installation

**Prerequisites:** Rust 2021+ ([install here](https://rustup.rs/))

```bash
# Clone the repository
git clone https://github.com/abinesha312/mailout.git
cd mailout

# Build and install
cargo install --path .

# Or run directly
cargo build --release
./target/release/mailout --help
```

## Configuration

Copy `.env.example` to `.env` and configure:

```bash
# Optional: Bright Data Web Unlocker API token
# Helps bypass LinkedIn's login wall on public profiles
# Sign up at https://brightdata.com
BRIGHTDATA_API_TOKEN=your_token_here

# Database location (default: ~/.mailout/mailout.db)
MAILOUT_DB=/path/to/mailout.db

# Log level (default: info)
RUST_LOG=info
```

**Without `BRIGHTDATA_API_TOKEN`:** LinkedIn often shows a login wall, which means profile data will be limited or empty.

## Usage

### CLI Lookup

```bash
# Basic lookup (username)
mailout lookup aharido

# Lookup with full URL
mailout lookup https://www.linkedin.com/in/aharido

# JSON output
mailout lookup aharido --json
```

**Example output:**

```
============================================================
LinkedIn Profile
============================================================
URL:      https://www.linkedin.com/in/aharido
Name:     John Doe
Headline: Software Engineer at Example Corp
Company:  Example Corp
Domain:   example.com
Location: San Francisco Bay Area

============================================================
Email Candidates (3)
============================================================

1. john.doe@example.com
   Type:       Work
   Source:     pattern
   Confidence: Guessed
   Verified:   false

2. jdoe@example.com
   Type:       Work
   Source:     web_search
   Confidence: Medium
   Verified:   false

3. john@example.com
   Type:       Work
   Source:     pattern
   Confidence: Guessed
   Verified:   false
```

### HTTP API Server

```bash
# Start server on default port 3000
mailout serve

# Custom port
mailout serve --port 8080
```

**API endpoint:**

```bash
curl -X POST http://localhost:3000/v1/lookup \
  -H "Content-Type: application/json" \
  -d '{"linkedin_url": "aharido"}'
```

**Response:**

```json
{
  "success": true,
  "result": {
    "profile": {
      "slug": "aharido",
      "url": "https://www.linkedin.com/in/aharido",
      "full_name": "John Doe",
      "headline": "Software Engineer",
      "company": "Example Corp",
      "company_domain": "example.com",
      "location": "San Francisco, CA"
    },
    "emails": [
      {
        "address": "john.doe@example.com",
        "type": "work",
        "source": "pattern",
        "confidence": "guessed",
        "verified": false
      }
    ],
    "errors": [],
    "cached": false
  }
}
```

### Database Stats

```bash
mailout db stats
```

**Output:**

```
Database: /home/user/.mailout/mailout.db
People:   42
Emails:   138
Lookups:  67
Pages:    156
```

## How It Works

### 1. LinkedIn Profile Scraping

- Fetches the public profile page via Bright Data Web Unlocker (if token provided) or direct HTTP
- Parses HTML to extract: name, headline, company, location, website
- Looks for any emails directly visible on the page (rare)
- LinkedIn often requires login for full access; this tool can only access what's publicly visible

### 2. Web Search for Emails

- Searches DuckDuckGo with queries like:
  - `"Full Name" email`
  - `"Full Name" @company.com`
  - `site:github.com "Full Name"`
- Fetches company pages (`/about`, `/contact`, `/team`) and extracts emails
- Parses emails from search results and pages using regex and HTML parsing

### 3. Pattern Generation

When company domain is known and name is known:
- Generates common patterns: `first.last@domain`, `firstlast@domain`, `f.last@domain`, etc.
- Labels these as **guessed** (not verified)
- Returns top 3 patterns

### 4. Database Caching

All data is stored in a local SQLite database (`~/.mailout/mailout.db`):
- **people:** LinkedIn profiles with raw JSON
- **emails:** Candidate emails with source/confidence
- **pages:** Fetched pages with status
- **lookups:** Query history

Repeat lookups check the cache first (24-hour TTL).

## Limitations & Honesty

1. **LinkedIn profiles rarely show emails.** This is not a tool limitation; it's how LinkedIn works.
2. **Pattern emails are guesses.** They may not exist or may bounce.
3. **No email verification.** We don't probe mail servers (that can look like an attack).
4. **Login walls.** Without Bright Data token, LinkedIn often blocks public access.
5. **Web search is best-effort.** Public pages may not have emails.
6. **Not a data broker.** We don't have a database of billions of emails; we fetch/search in real-time.

## Legal & Ethical Use

This tool is for **professional B2B contact enrichment** only:

- ✅ Finding work emails for legitimate outreach (sales, recruiting, partnerships)
- ✅ One-off lookups for specific people
- ✅ Respecting opt-outs and unsubscribes
- ❌ Bulk scraping LinkedIn profiles
- ❌ Spam, robocalls, or harassment
- ❌ Violating CAN-SPAM, GDPR, or local laws
- ❌ Using data from breaches or dumps

**You are responsible for complying with all applicable laws** (CAN-SPAM, GDPR, CCPA, etc.).

## Development

```bash
# Run tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Format code
cargo fmt

# Lint
cargo clippy

# Build release
cargo build --release
```

## License

MIT License - see [LICENSE](LICENSE)

## Contributing

Contributions welcome! Please open an issue or PR.

## Disclaimer

MailOut is provided "as is" without warranty of any kind. The authors are not responsible for misuse, legal violations, or damages resulting from use of this software. Use at your own risk and comply with all applicable laws.
