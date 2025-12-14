import csv
import requests
import os
from dotenv import load_dotenv
import base64
from datetime import datetime, timedelta

load_dotenv()


def load_songs_from_csv(filepath):
    songs = []
    with open(filepath, "r", encoding="utf-8") as f:
        reader = csv.DictReader(
            f, fieldnames=["id", "song_name", "artist", "uri", "colors"]
        )
        for row in reader:
            # Split the colors string into a list and strip whitespace
            colors = (
                [color.strip().capitalize() for color in row["colors"].split(";")]
                if row["colors"]
                else []
            )

            song_entry = {
                "id": int(row["id"]),
                "song_name": row["song_name"].strip(),
                "artist": row["artist"].strip(),
                "uri": row["uri"].strip(),
                "colors": colors,
            }
            songs.append(song_entry)
    return songs


def save_songs_to_csv(songs, filepath):
    with open(filepath, "w", newline="", encoding="utf-8") as f:
        writer = csv.DictWriter(
            f, fieldnames=["id", "song_name", "artist", "uri", "colors"]
        )
        for song in songs:
            # Convert the colors list to a semicolon-separated string
            song_row = song.copy()
            song_row["colors"] = ";".join(song["colors"])
            writer.writerow(song_row)


class SpotifyController:
    def __init__(self):
        self.base_url = "https://api.spotify.com/v1"
        self.client_id = os.getenv("SPOTIFY_CLIENT_ID")
        self.client_secret = os.getenv("SPOTIFY_CLIENT_SECRET")
        self.refresh_token = os.getenv("SPOTIFY_REFRESH_TOKEN")
        self.token = None
        self.token_expiry = None
        self._get_access_token()

    def _get_access_token(self):
        auth = base64.b64encode(
            f"{self.client_id}:{self.client_secret}".encode()
        ).decode()
        response = requests.post(
            "https://accounts.spotify.com/api/token",
            headers={"Authorization": f"Basic {auth}"},
            data={"grant_type": "refresh_token", "refresh_token": self.refresh_token},
        )
        if response.status_code == 200:
            data = response.json()
            self.token = data["access_token"]
            self.token_expiry = datetime.now() + timedelta(seconds=data["expires_in"])

    def _check_token(self):
        if not self.token or datetime.now() >= self.token_expiry:
            self._get_access_token()

    def get_playlist_tracks(self, playlist_url, default_colors=None):
        """
        Get all tracks from a Spotify playlist and format them for CSV storage

        Parameters:
        playlist_url (str): Full Spotify playlist URL or just the playlist ID
        default_colors (list): Default colors to assign to tracks, defaults to ["White"]

        Returns:
        list: List of dictionaries containing track information with IDs
        """
        self._check_token()

        if default_colors is None:
            default_colors = ["White"]

        if "playlist/" in playlist_url:
            playlist_id = playlist_url.split("playlist/")[1].split("?")[0]
        else:
            playlist_id = playlist_url

        tracks = []
        offset = 0
        limit = 100
        track_id = 1

        while True:
            response = requests.get(
                f"{self.base_url}/playlists/{playlist_id}/tracks",
                headers={"Authorization": f"Bearer {self.token}"},
                params={
                    "offset": offset,
                    "limit": limit,
                    "fields": "items(track(name,uri,artists(name))),total,next",
                },
            )

            if response.status_code != 200:
                print(f"Error fetching tracks: {response.status_code}")
                return None

            data = response.json()

            for item in data["items"]:
                if item["track"]:
                    track_info = {
                        "id": track_id,
                        "song_name": item["track"]["name"],
                        "artist": ", ".join(
                            artist["name"] for artist in item["track"]["artists"]
                        ),
                        "uri": item["track"]["uri"],
                        "colors": default_colors.copy(),  # Use copy to avoid sharing the same list
                    }
                    tracks.append(track_info)
                    track_id += 1

            if not data.get("next"):
                break

            offset += limit

        return tracks


if __name__ == "__main__":
    controller = SpotifyController()

    playlist_url = input("Enter Spotify playlist URL: ")
    output_file = input("Enter output CSV filename: ")

    # Example with multiple default colors
    default_colors = ["White", "Blue"]
    tracks = controller.get_playlist_tracks(playlist_url, default_colors)

    if tracks:
        print(f"\nFound {len(tracks)} tracks in playlist:")
        for track in tracks:
            print(f"{track['id']}. {track['song_name']} by {track['artist']}")
            print(f"   URI: {track['uri']}")
            print(f"   Colors: {', '.join(track['colors'])}")
            print("-" * 50)

        save_songs_to_csv(tracks, output_file)
        print(f"\nTracks saved to {output_file}")

        # Verify the save by loading and displaying
        loaded_tracks = load_songs_from_csv(output_file)
        print(f"\nVerified {len(loaded_tracks)} tracks loaded from CSV successfully")

        # Show an example of the loaded data
        if loaded_tracks:
            print("\nExample of loaded track:")
            track = loaded_tracks[0]
            print(f"ID: {track['id']}")
            print(f"Song: {track['song_name']}")
            print(f"Artist: {track['artist']}")
            print(f"URI: {track['uri']}")
            print(f"Colors: {track['colors']}")
