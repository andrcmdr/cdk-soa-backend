use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tokio;

// Twitter API structures
#[derive(Debug, Deserialize)]
struct TwitterUserResponse {
    data: TwitterUser,
}

#[derive(Debug, Deserialize)]
struct TwitterUser {
    id: String,
    name: String,
    username: String,
}

// Discord API structures
#[derive(Debug, Deserialize)]
struct DiscordUser {
    id: String,
    username: String,
    discriminator: String,
}

// Google API structures
#[derive(Debug, Deserialize)]
struct GooglePersonResponse {
    resourceName: String,
    emailAddresses: Option<Vec<GoogleEmail>>,
}

#[derive(Debug, Deserialize)]
struct GoogleEmail {
    metadata: GoogleEmailMetadata,
    value: String,
}

#[derive(Debug, Deserialize)]
struct GoogleEmailMetadata {
    primary: Option<bool>,
}

// Configuration structure
#[derive(Debug, Deserialize)]
struct Config {
    twitter_bearer_token: Option<String>,
    discord_bot_token: Option<String>,
    google_access_token: Option<String>,
}

// Result structure
#[derive(Debug, Serialize)]
struct UserIdResult {
    platform: String,
    username: String,
    user_id: Option<String>,
    error: Option<String>,
}

struct ApiClient {
    client: Client,
    config: Config,
}

impl ApiClient {
    fn new(config: Config) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }

    /// Fetch Twitter user ID by username
    async fn get_twitter_user_id(&self, username: &str) -> Result<String> {
        let bearer_token = self
            .config
            .twitter_bearer_token
            .as_ref()
            .context("Twitter bearer token not configured")?;

        let url = format!(
            "https://api.twitter.com/2/users/by/username/{}",
            username.trim_start_matches('@')
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", bearer_token))
            .send()
            .await
            .context("Failed to send Twitter API request")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            anyhow::bail!("Twitter API error ({}): {}", status, text);
        }

        let user_response: TwitterUserResponse = response
            .json()
            .await
            .context("Failed to parse Twitter response")?;

        Ok(user_response.data.id)
    }

    /// Fetch Discord user ID by username
    /// Note: This is limited and only works if the bot shares a server with the user
    async fn get_discord_user_id(&self, username: &str) -> Result<String> {
        anyhow::bail!(
            "Discord API doesn't support global user lookup by username. \
             User IDs can only be retrieved if the bot shares a server with the user. \
             Use Discord's client with developer mode enabled to manually copy user IDs."
        );
    }

    /// Fetch Google user ID by email
    /// Note: This requires OAuth flow and is limited
    async fn get_google_user_id(&self, email: &str) -> Result<String> {
        let access_token = self
            .config
            .google_access_token
            .as_ref()
            .context("Google access token not configured")?;

        // Google doesn't provide direct email-to-user-ID lookup
        // This would require Directory API with admin privileges
        // For Gmail API, you can only get info about the authenticated user
        anyhow::bail!(
            "Google APIs don't support looking up user IDs by arbitrary email addresses. \
             You can only get information about the currently authenticated user using 'me' as the user ID. \
             For organization-wide lookups, use Google Workspace Admin SDK Directory API."
        );
    }

    /// Fetch user ID from specified platform
    async fn get_user_id(&self, platform: &str, username: &str) -> UserIdResult {
        let result = match platform.to_lowercase().as_str() {
            "twitter" | "x" => self.get_twitter_user_id(username).await,
            "discord" => self.get_discord_user_id(username).await,
            "google" | "gmail" => self.get_google_user_id(username).await,
            _ => Err(anyhow::anyhow!("Unknown platform: {}", platform)),
        };

        match result {
            Ok(user_id) => UserIdResult {
                platform: platform.to_string(),
                username: username.to_string(),
                user_id: Some(user_id),
                error: None,
            },
            Err(e) => UserIdResult {
                platform: platform.to_string(),
                username: username.to_string(),
                user_id: None,
                error: Some(e.to_string()),
            },
        }
    }
}

// User list entry structure
#[derive(Debug, Deserialize)]
struct UserEntry {
    platform: String,
    username: String,
}

/// Load configuration from file or environment variables
fn load_config() -> Result<Config> {
    // Try to load from config.json first
    let config_path = PathBuf::from("config.json");
    if config_path.exists() {
        let config_str = fs::read_to_string(&config_path)
            .context("Failed to read config.json")?;
        let config: Config = serde_json::from_str(&config_str)
            .context("Failed to parse config.json")?;
        return Ok(config);
    }

    // Fall back to environment variables
    Ok(Config {
        twitter_bearer_token: std::env::var("TWITTER_BEARER_TOKEN").ok(),
        discord_bot_token: std::env::var("DISCORD_BOT_TOKEN").ok(),
        google_access_token: std::env::var("GOOGLE_ACCESS_TOKEN").ok(),
    })
}

/// Load user list from JSON file
fn load_user_list(path: &PathBuf) -> Result<Vec<UserEntry>> {
    let content = fs::read_to_string(path)
        .context("Failed to read user list file")?;
    let users: Vec<UserEntry> = serde_json::from_str(&content)
        .context("Failed to parse user list JSON")?;
    Ok(users)
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <userlist.json>", args[0]);
        eprintln!("\nExpected format for userlist.json:");
        eprintln!(r#"[
  {{"platform": "twitter", "username": "elonmusk"}},
  {{"platform": "discord", "username": "user#1234"}},
  {{"platform": "google", "username": "user@gmail.com"}}
]"#);
        eprintln!("\nConfiguration:");
        eprintln!("Create config.json with:");
        eprintln!(r#"{{
  "twitter_bearer_token": "YOUR_BEARER_TOKEN",
  "discord_bot_token": "YOUR_BOT_TOKEN",
  "google_access_token": "YOUR_ACCESS_TOKEN"
}}"#);
        eprintln!("\nOr set environment variables:");
        eprintln!("  TWITTER_BEARER_TOKEN");
        eprintln!("  DISCORD_BOT_TOKEN");
        eprintln!("  GOOGLE_ACCESS_TOKEN");
        std::process::exit(1);
    }

    let userlist_path = PathBuf::from(&args[1]);

    // Load configuration
    let config = load_config()
        .context("Failed to load configuration")?;

    // Load user list
    let users = load_user_list(&userlist_path)
        .context("Failed to load user list")?;

    tracing::info!("Loaded {} users from {:?}", users.len(), userlist_path);

    // Create API client
    let client = ApiClient::new(config);

    // Fetch user IDs
    let mut results = Vec::new();
    for user in users {
        tracing::info!("Fetching user ID for {} on {}", user.username, user.platform);
        let result = client.get_user_id(&user.platform, &user.username).await;

        match &result.user_id {
            Some(id) => tracing::info!("✓ Found ID: {}", id),
            None => tracing::warn!("✗ Error: {}", result.error.as_ref().unwrap()),
        }

        results.push(result);
    }

    // Output results as JSON
    let output = serde_json::to_string_pretty(&results)?;
    println!("\n{}", output);

    // Save results to file
    let output_path = PathBuf::from("results.json");
    fs::write(&output_path, output)
        .context("Failed to write results.json")?;
    tracing::info!("Results saved to {:?}", output_path);

    Ok(())
}
