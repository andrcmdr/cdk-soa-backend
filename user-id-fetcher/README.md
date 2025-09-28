# User ID Fetcher CLI

A command-line tool to fetch unique user IDs by username from Twitter (X), Discord, and Google/Gmail.

## Features

- Fetch Twitter/X user IDs using Twitter API v2
- Support for batch processing via JSON user list
- JSON output with detailed error messages
- Configurable via config file or environment variables

## Important Limitations

### Twitter/X
- ✅ **Works**: Can fetch user IDs by username using Bearer Token
- Requires Twitter API v2 access and Bearer Token
- Rate limits apply based on your API tier

### Discord
- ❌ **Limited**: Discord API doesn't support global user lookup by username
- User IDs can only be retrieved if your bot shares a server with the user
- Recommended: Use Discord client with Developer Mode to manually copy user IDs

### Google/Gmail
- ❌ **Limited**: Google APIs don't support arbitrary email-to-user-ID lookup
- Can only get info about the currently authenticated user
- For organization-wide lookups, use Google Workspace Admin SDK Directory API

## Setup

### 1. Get API Credentials

#### Twitter (X)
1. Go to [Twitter Developer Portal](https://developer.twitter.com/)
2. Create a new project and app
3. Generate a Bearer Token
4. Copy the Bearer Token

#### Discord
1. Go to [Discord Developer Portal](https://discord.com/developers/applications)
2. Create a new application
3. Go to "Bot" section and create a bot
4. Copy the bot token

#### Google
1. Go to [Google Cloud Console](https://console.cloud.google.com/)
2. Create a new project
3. Enable People API or Gmail API
4. Create OAuth 2.0 credentials
5. Get an access token via OAuth flow

### 2. Configure the Tool

Create a `config.json` file:

```json
{
  "twitter_bearer_token": "YOUR_TWITTER_BEARER_TOKEN",
  "discord_bot_token": "YOUR_DISCORD_BOT_TOKEN",
  "google_access_token": "YOUR_GOOGLE_ACCESS_TOKEN"
}
```

Or set environment variables:
```bash
export TWITTER_BEARER_TOKEN="your_token"
export DISCORD_BOT_TOKEN="your_token"
export GOOGLE_ACCESS_TOKEN="your_token"
```

### 3. Create User List

Create a `userlist.json` file:

```json
[
  {
    "platform": "twitter",
    "username": "elonmusk"
  },
  {
    "platform": "twitter",
    "username": "@jack"
  }
]
```

## Usage

```bash
# Build the project
cargo build --release

# Run the tool
cargo run --release userlist.json

# Or use the binary directly
./target/release/user-id-fetcher userlist.json
```

## Output

The tool outputs results in JSON format and saves them to `results.json`:

```json
[
  {
    "platform": "twitter",
    "username": "elonmusk",
    "user_id": "44196397",
    "error": null
  },
  {
    "platform": "discord",
    "username": "someuser#1234",
    "user_id": null,
    "error": "Discord API doesn't support global user lookup by username..."
  }
]
```

## API Documentation References

- [Twitter API v2 - Users Lookup](https://developer.twitter.com/en/docs/twitter-api/users/lookup/api-reference/get-users-by-username-username)
- [Discord API - Users Resource](https://discord.com/developers/docs/resources/user)
- [Google People API](https://developers.google.com/people)

## License

This project is licensed under the [Apache 2.0 License](LICENSE-APACHE).

## How to Use

1. **Build the project**:
   ```bash
   cargo build --release
   ```

2. **Set up your credentials** in `config.json` or environment variables

3. **Create your user list** in `userlist.json`

4. **Run the tool**:
   ```bash
   cargo run --release userlist.json
   ```

The tool will:
- Load the configuration and user list
- Fetch user IDs for each entry (where supported)
- Display progress with logging
- Output results as JSON
- Save results to `results.json`

**Note**: Due to API limitations, only Twitter/X fully supports username-to-ID lookup.
Discord and Google have significant restrictions that make programmatic lookup challenging or impossible without special access.

