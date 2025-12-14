import json
import time
import io
import gzip
import subprocess
import os
import sys
import requests
from pathlib import Path

YT_BASE_URL = "https://www.youtube.com/watch?v="


def get_config(name: str, *, required: bool = False) -> str | None:
    cred_dir = os.environ.get("CREDENTIALS_DIRECTORY")
    if cred_dir:
        p = Path(cred_dir) / name
        if p.exists():
            return p.read_text().strip()

    value = os.environ.get(name)
    if value:
        return value

    if required:
        raise RuntimeError(f"Missing required config value: {name}")

    return None


INPUT_JSON_PATH = get_config("INPUT_JSON_PATH")
INPUT_JSON_URL = get_config("INPUT_JSON_URL")
TELEGRAM_BOT_TOKEN = get_config("TELEGRAM_BOT_TOKEN", required=True)
TELEGRAM_CHAT_ID = get_config("TELEGRAM_CHAT_ID", required=True)


def send_telegram_message(text: str):
    if not TELEGRAM_BOT_TOKEN or not TELEGRAM_CHAT_ID:
        print("Telegram credentials not set. Skipping Telegram notification.")
        return

    url = f"https://api.telegram.org/bot{TELEGRAM_BOT_TOKEN}/sendMessage"
    payload = {
        "chat_id": TELEGRAM_CHAT_ID,
        "text": text,
        "disable_web_page_preview": True,
    }

    try:
        response = requests.post(url, json=payload, timeout=10)
        response.raise_for_status()
    except requests.RequestException as e:
        print(f"Failed to send Telegram message: {e}")


def send_failure_notification(stage: str, error: Exception):
    message = f"🚨 Script failure detected\n\nStage: {stage}\nError: {str(error)}"
    send_telegram_message(message)


def load_input_json():
    if INPUT_JSON_PATH and INPUT_JSON_URL:
        raise RuntimeError(
            "Both INPUT_JSON_PATH and INPUT_JSON_URL are set. Choose only one."
        )

    if INPUT_JSON_PATH:
        if not os.path.exists(INPUT_JSON_PATH):
            raise FileNotFoundError(f"Input file not found: {INPUT_JSON_PATH}")

        with open(INPUT_JSON_PATH, "r", encoding="utf-8") as f:
            return json.load(f)

    if INPUT_JSON_URL:
        if not INPUT_JSON_URL.startswith("https://"):
            raise ValueError("INPUT_JSON_URL must start with https://")

        cache_buster = int(time.time())
        url = f"{INPUT_JSON_URL}?cb={cache_buster}"

        headers = {
            "Cache-Control": "no-cache, no-store, must-revalidate",
            "Pragma": "no-cache",
            "Accept-Encoding": "gzip",
        }

        response = requests.get(url, headers=headers, timeout=20)
        response.raise_for_status()

        content_type = response.headers.get("Content-Type", "")
        content_encoding = response.headers.get("Content-Encoding", "")
        raw_bytes = response.content

        if "application/json" in content_type and not content_encoding:
            return json.loads(raw_bytes.decode("utf-8"))

        try:
            with gzip.GzipFile(fileobj=io.BytesIO(raw_bytes)) as gz:
                decompressed = gz.read().decode("utf-8")
                return json.loads(decompressed)
        except OSError as e:
            raise RuntimeError("Failed to decompress gzipped JSON") from e

    raise RuntimeError(
        "No input source configured. Set INPUT_JSON_PATH or INPUT_JSON_URL."
    )


def check_video_availability(video_id: str) -> bool:
    url = f"{YT_BASE_URL}{video_id}"

    try:
        subprocess.check_call(
            ["yt-dlp", "--list-formats", "--simulate", "--no-warnings", url],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        return True
    except subprocess.CalledProcessError:
        return False
    except FileNotFoundError:
        raise RuntimeError("yt-dlp not found in PATH")


def format_unavailable_songs_message(unavailable_songs):
    if not unavailable_songs:
        return "✅ YouTube availability check finished.\n\nAll videos are available."

    lines = [
        "❌ YouTube availability check finished.",
        f"Unavailable videos: {len(unavailable_songs)}",
        "",
    ]

    for song in unavailable_songs:
        lines.append(f"• {song['title']} – {song['artist']}\n  {song['url']}")

    return "\n".join(lines)


def main():
    try:
        data = load_input_json()
    except Exception as e:
        send_failure_notification("loading input JSON", e)
        raise

    media_list = data.get("media", [])
    unavailable_songs = []

    total_count = len(media_list)
    print(f"Loaded {total_count} media entries. Starting check...\n")

    try:
        for index, item in enumerate(media_list, 1):
            video_id = item.get("youtube_id")
            title = item.get("title", "Unknown Title")
            artist = item.get("artist", "Unknown Artist")

            if not video_id or video_id == "TEMP":
                print(f"[{index}/{total_count}] ⚠️  Skipping (No ID): {title}")
                continue

            sys.stdout.write(
                f"[{index}/{total_count}] Checking: {title} ({video_id})... "
            )
            sys.stdout.flush()

            is_available = check_video_availability(video_id)

            if is_available:
                print("✅ Available")
            else:
                print("❌ UNAVAILABLE")
                unavailable_songs.append(
                    {
                        "id": item.get("id"),
                        "title": title,
                        "artist": artist,
                        "youtube_id": video_id,
                        "url": f"{YT_BASE_URL}{video_id}",
                    }
                )

    except Exception as e:
        send_failure_notification("checking YouTube availability", e)
        raise

    message = format_unavailable_songs_message(unavailable_songs)
    send_telegram_message(message)


if __name__ == "__main__":
    try:
        main()
    except Exception:
        sys.exit(1)
