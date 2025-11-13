# 🎵 Spektrum

An interactive multiplayer music quiz where players compete to identify songs through colors and characters.

https://github.com/user-attachments/assets/4d27e01c-d5c3-4465-83c7-8fcd82cc5a73

(*sound on* ↑)

## How to Play

Spektrum is meant to be played with all participants in the same room. One person acts as the host and is responsible for music playback. The host creates a lobby and shares the join code with the group. When players have joined, the host starts the game.

Players will hear a piece of music and must identify the correct color or character associated with each song. Quick answers score more points.

## Current Questions

The quiz at https://melodiquiz.se features a mix of Anglosphere and Swedish music. If you'd prefer to use your own question set, you can host your own instance following the instructions below.

## Host Your Own

### Backend Setup

#### Local Development (Filesystem Storage)
```bash
git clone https://github.com/kottz/spektrum
cd server
```

Set the following environment variables:

**Public Configuration:**
- `SPEKTRUM__SERVER__PORT=8765`
- `SPEKTRUM__SERVER__CORS_ORIGINS=http://localhost:5173` (comma-separated for multiple)
- `SPEKTRUM__LOGGING__JSON=false`
- `SPEKTRUM__STORAGE__TYPE=filesystem`
- `SPEKTRUM__STORAGE__BASE_PATH=data`
- `SPEKTRUM__STORAGE__FILE_PATH=questions.json`

**Secrets:**
- `SPEKTRUM__ADMIN_PASSWORD=your-password-here` (comma-separated for multiple)

```bash
cargo run --release
```

#### Production Deployment (S3/B2 Storage)
There is a provided Dockerfile that pairs for deployment with fly.io or similar platforms. The question database can be stored in S3-compatible storage (like Backblaze B2):

1. Create a public Backblaze B2 bucket
2. Set the following environment variables in your deployment platform:

**Public Configuration:**
- `SPEKTRUM__SERVER__PORT=8765`
- `SPEKTRUM__SERVER__CORS_ORIGINS=https://your-domain.com` (comma-separated list)
- `SPEKTRUM__LOGGING__JSON=true` (recommended for production)
- `SPEKTRUM__STORAGE__TYPE=s3`
- `SPEKTRUM__STORAGE__BUCKET=your-bucket-name`
- `SPEKTRUM__STORAGE__REGION=your-region` (e.g., eu-central-003)
- `SPEKTRUM__STORAGE__PREFIX=data`
- `SPEKTRUM__STORAGE__QUESTION_FOLDER=question_folder_name`
- `SPEKTRUM__STORAGE__QUESTION_FILE=questions.json`

**Secrets:**
- `SPEKTRUM__ADMIN_PASSWORD=your-secure-password` (comma-separated for multiple)
- `SPEKTRUM__STORAGE__ACCESS_KEY_ID=your-b2-key-id`
- `SPEKTRUM__STORAGE__SECRET_ACCESS_KEY=your-b2-secret-key`

See `server/env.example` for a complete configuration template.

With this example configuration your questions will be stored in `bucket_root/data/question_folder_name/questions.json`.

### Frontend Setup

#### Configuration
```bash
cd frontend
cp env.example .env.development
cp env.example .env.production
```

Environment variables:
- `PUBLIC_SPEKTRUM_SERVER_URL`: Backend HTTP endpoint
- `PUBLIC_SPEKTRUM_WS_SERVER_URL`: Backend WebSocket endpoint
- `PUBLIC_SPEKTRUM_CDN_URL`: CDN URL for media (optional)
- `PUBLIC_UMAMI_WEBSITE_ID`: Analytics ID (optional)
- `PUBLIC_TITLE`: Application title

See `frontend/env.example` for details.

#### Running
```bash
npm run dev --host    # Development
npm run build         # Production
```

### Creating Custom Questions

There is also a separate admin panel you can use as a convenient way to add and remove questions and sets.

#### Configuration
```bash
cd admin_panel
cp env.example .env.development
cp env.example .env.production
```

Environment variables:
- `PUBLIC_SPEKTRUM_SERVER_URL`: Backend HTTP endpoint
- `PUBLIC_SPEKTRUM_WS_SERVER_URL`: Backend WebSocket endpoint
- `PUBLIC_SPEKTRUM_CDN_URL`: CDN URL for media (optional)
- `PUBLIC_DEV_ADMIN_PASSWORD`: Auto-login for dev mode (optional)

See `admin_panel/env.example` for details.

#### Running
```bash
npm run dev --host    # Development
npm run build         # Production
```
Questions are stored in `server/data` as a JSON file. The game supports two types of questions:

**Color** questions require:
- Media (Song with a YouTube-link)
- One or more colors from: Red, Green, Blue, Yellow, Purple, Gold, Silver, Pink, Black, White, Brown, Orange, Gray

**Character** questions require:
- Media (Song with a YouTube-link)
- Six different character options per question
- 300x300 AVIF image for each character
