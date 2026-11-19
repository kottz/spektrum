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

```bash
git clone https://github.com/kottz/spektrum
cd server
cp env.example .env
set -a && . ./.env && set +a   # the server reads its settings from the environment
cargo run --release
```

See `env.example` for configuration options. With [just](https://github.com/casey/just) installed, `just dev-server` does the same in debug mode, and `just check` runs every check CI runs.

### Frontend Setup

```bash
cd frontend
cp env.example .env.development
cp env.example .env.production
npm run dev -- --host # Development
npm run build         # Production
```

See `env.example` for configuration options.

### Creating Custom Questions

There is also a separate admin panel you can use as a convenient way to add and remove questions and sets.

```bash
cd admin_panel
cp env.example .env.development
cp env.example .env.production
npm run dev -- --host # Development
npm run build         # Production
```

See `env.example` for configuration options.
Questions are stored in `server/data` as a JSON file. The game supports four types of questions:

**Color** questions require:
- Media (Song with a YouTube-link)
- One or more colors from: Red, Green, Blue, Yellow, Purple, Gold, Silver, Pink, Black, White, Brown, Orange, Gray

**Character** questions require:
- Media (Song with a YouTube-link)
- Six different character options per question
- 300x300 AVIF image for each character

**Text** questions require:
- Media (Song with a YouTube-link)
- Question text and text answer options, at least one marked correct

**Year** questions require:
- Media (Song with a YouTube-link)
- One correct option holding the year; players pick from that year ±2
