# 🎵 Spektrum

An interactive multiplayer music quiz where players compete to identify songs through colors and characters.

## How to Play

Spektrum is meant to be played with all participants in the same room. One person acts as the host and is responsible for music playback. The host creates a lobby and shares the join code with the group. When players have joined, the host starts the game.

Players will hear a piece of music and must identify the correct color or character associated with each song. Quick answers score more points.

## Current Questions

The quiz at https://quiz.kottz.se features a mix of Anglosphere and Swedish music. If you'd prefer to use your own question set, you can host your own instance following the instructions below.

## Host Your Own

### Backend Setup
```bash
git clone https://github.com/kottz/spektrum
cd server
cp config.example.toml config.toml  # Edit as needed
cargo run --release
```

If you want to deploy your own instance there is a provided Dockerfile that pairs well with fly.io.

### Frontend Setup
```bash
cd frontend
npm run dev --host    # Development
npm run build         # Production
```

### Creating Custom Questions

Questions are stored in `server/data` as CSV files. The game supports two types of questions:

Color questions require:
- Song title and artist name
- One or more colors from: Red, Green, Blue, Yellow, Purple, Gold, Silver, Pink, Black, White, Brown, Orange, Gray
- YouTube link for playback

Character questions require:
- Song title
- One correct character name
- Five alternative character names
- YouTube link
- 300x300 AVIF image for each character in `static/img`, filename matching the character name in CSV
