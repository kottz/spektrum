#!/usr/bin/env python3
import json
import sys
import time
import requests


def check_youtube_video(video_id):
    url = f"https://www.youtube.com/oembed?url=https://www.youtube.com/watch?v={video_id}&format=json"
    try:
        response = requests.get(url, timeout=10)
        return response.status_code == 200
    except requests.RequestException:
        return False


def main():
    if len(sys.argv) != 2:
        print("Usage: python check_yt_link.py songs.json")
        sys.exit(1)

    input_file = sys.argv[1]
    with open(input_file, "r", encoding="utf-8") as f:
        data = json.load(f)

    results = []
    for entry in data.get("media", []):
        video_id = entry.get("youtube_id")
        title = entry.get("title")
        artist = entry.get("artist")

        available = check_youtube_video(video_id)
        status = "✅ Available" if available else "❌ Unavailable"
        print(f"{status} | {title} - {artist} ({video_id})")
        results.append(
            {
                "id": entry.get("id"),
                "title": title,
                "artist": artist,
                "youtube_id": video_id,
                "available": available,
            }
        )
        time.sleep(1)  # 1 request per second rate limit

    # Save results to a file
    output_file = "youtube_check_results.json"
    with open(output_file, "w", encoding="utf-8") as f:
        json.dump(results, f, ensure_ascii=False, indent=2)

    print(f"\nResults saved to {output_file}")


if __name__ == "__main__":
    main()
