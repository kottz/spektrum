#!/usr/bin/env python3
"""
spotify_playlist_to_youtube_csv.py

Fetch every track from a Spotify playlist and export a CSV with the following
columns:

    Title, Artist, Release Year, Spotify ID, YouTube link ID

Dependencies
------------
- spotipy : https://spotipy.readthedocs.io
- ytmusicapi : https://ytmusicapi.readthedocs.io

Environment Variables
---------------------
- SPOTIFY_CLIENT_ID
- SPOTIFY_CLIENT_SECRET

For YouTube Music access, place a valid ``headers_auth.json`` in the working
folder (see ytmusicapi docs) or supply authentication parameters supported by
``YTMusic``.
"""

from __future__ import annotations

import argparse
import csv
import os
from datetime import datetime
from typing import Dict, List, Optional

import spotipy
from spotipy import SpotifyClientCredentials
from ytmusicapi import YTMusic


class SpotifyPlaylistConverter:
    """Convert a Spotify playlist to a CSV enriched with YouTube Music IDs."""

    def __init__(self) -> None:
        # --- Spotify client setup -------------------------------------------------
        credentials = SpotifyClientCredentials(
            client_id=os.getenv("SPOTIFY_CLIENT_ID"),
            client_secret=os.getenv("SPOTIFY_CLIENT_SECRET"),
        )
        self.spotify = spotipy.Spotify(auth_manager=credentials)

        # --- YouTube Music client setup ------------------------------------------
        # ``YTMusic()`` expects ``headers_auth.json`` next to the script when no
        # arguments are supplied.
        self.ytmusic = YTMusic()

    # -------------------------------------------------------------------------
    # Spotify helpers
    # -------------------------------------------------------------------------
    @staticmethod
    def _extract_playlist_id(raw: str) -> str:
        """Return the bare playlist ID from a Spotify URL or URI."""
        if "playlist/" in raw:
            return raw.split("playlist/")[1].split("?")[0]
        return raw.split(":")[-1]

    def _fetch_playlist_tracks(self, playlist_id: str) -> List[Dict]:
        """Retrieve *all* track objects from the playlist (handles pagination)."""
        tracks: List[Dict] = []
        offset = 0
        limit = 100
        while True:
            page = self.spotify.playlist_items(playlist_id, limit=limit, offset=offset)
            tracks.extend(
                [item["track"] for item in page["items"] if item.get("track")]
            )
            if page["next"] is None:
                break
            offset += limit
        return tracks

    @staticmethod
    def _parse_track_data(track: Dict) -> Dict:
        """Return the core metadata we need for a row."""
        title = track["name"]
        artist = track["artists"][0]["name"]
        spotify_id = track["id"]
        # Spotify stores release dates on the *album* object. Usually YYYY-MM-DD
        release_date = track["album"].get("release_date", "")
        year = release_date[:4] if release_date else ""
        return {
            "Title": title,
            "Artist": artist,
            "Release Year": year,
            "Spotify ID": spotify_id,
        }

    # -------------------------------------------------------------------------
    # YouTube Music helper
    # -------------------------------------------------------------------------
    def _search_youtube_id(self, title: str, artist: str) -> Optional[str]:
        query = f"{title} {artist}"
        results = self.ytmusic.search(query, filter="songs", limit=1)
        return results[0]["videoId"] if results else None

    # -------------------------------------------------------------------------
    # Public API
    # -------------------------------------------------------------------------
    def convert(self, playlist: str, outfile: str) -> None:
        playlist_id = self._extract_playlist_id(playlist)
        print("Fetching playlist metadata from Spotify …")
        tracks = self._fetch_playlist_tracks(playlist_id)
        total = len(tracks)
        if not total:
            print("No tracks found – aborting.")
            return

        print(f"Found {total} tracks. Beginning conversion …")
        with open(outfile, "w", newline="", encoding="utf-8") as fh:
            writer = csv.DictWriter(
                fh,
                fieldnames=[
                    "Title",
                    "Artist",
                    "Release Year",
                    "Spotify ID",
                    "YouTube link ID",
                ],
            )
            writer.writeheader()

            for idx, track in enumerate(tracks, start=1):
                record = self._parse_track_data(track)
                yt_id = self._search_youtube_id(record["Title"], record["Artist"])
                record["YouTube link ID"] = yt_id or ""
                writer.writerow(record)

                if idx % 20 == 0 or idx == total:
                    print(f"Processed {idx}/{total} tracks …")

        print(f"✔︎ CSV written to '{outfile}'")


def _cli() -> None:
    parser = argparse.ArgumentParser(
        description="Export a Spotify playlist to CSV enriched with YouTube video IDs",
    )
    parser.add_argument(
        "playlist",
        help="Spotify playlist URL or URI",
    )
    parser.add_argument(
        "--output",
        "-o",
        default="playlist_export.csv",
        help="Destination CSV file (default: playlist_export.csv)",
    )
    args = parser.parse_args()

    # Verify required environment variables
    for var in ("SPOTIFY_CLIENT_ID", "SPOTIFY_CLIENT_SECRET"):
        if var not in os.environ:
            raise SystemExit(f"Environment variable '{var}' is not set. Aborting.")

    converter = SpotifyPlaylistConverter()
    converter.convert(args.playlist, args.output)


if __name__ == "__main__":
    _cli()
