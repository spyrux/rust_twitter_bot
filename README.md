# ASUKA
Asuka is an AI agent Built with [rig](https://github.com/0xPlaygrounds/rig/) and a fork of [Rina](https://github.com/cornip/Rina)

[TWITTER](https://x.com/Asuka_RIG)

## Added Features

- **Rig-Twitter Integration**
  - Cookie-based authentication
  - No Twitter API costs

- **Telegram Service**
  - Complete Telegram bot integration
  - Real-time messaging capabilities

- **Heuris Image Generator**
  - AI-powered image generation for tweets

- **Enhanced AI Agent Communication**
  - Pre-defined message examples
  - Customizable topics
  - Configurable communication styles

## Prerequisites

- Rust programming language
- Cargo package manager
- Git

## Getting Started

### Environment Variables
To get the cookie string, you need to:
1. Open Chrome DevTools (F12)
2. Go to Network tab
3. Select Fetch/XHR
4. Choose any request that starts with https://x.com/i/api/graphql/
5. In Request Headers, copy the cookie value
6. Paste it in your .env file

```env
# Twitter Configuration
TWITTER_USERNAME=your_username
TWITTER_PASSWORD=your_password
TWITTER_EMAIL=your_email
TWITTER_2FA_SECRET=your_2fa_secret
TWITTER_COOKIE_STRING=your_cookie_string

# Bot Tokens
TELEGRAM_BOT_TOKEN=your_telegram_token
DISCORD_API_TOKEN=your_discord_token

# API Keys
OPENAI_API_KEY=your_openai_key
HEURIST_API_KEY=your_heurist_key
```
## Usage

Start the service:
```bash
cargo run
```

## Credits

- Original project: [dojoengine/asuka](https://github.com/dojoengine/asuka)
- Additional features and modifications by [Rina](https://github.com/cornip/Rina)
"# rust_twitter_bot" 
