import requests
import os
from dotenv import load_dotenv
import base64
from datetime import datetime, timedelta
import time

load_dotenv()


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

    def search_track(self, query):
        self._check_token()
        response = requests.get(
            f"{self.base_url}/search",
            headers={"Authorization": f"Bearer {self.token}"},
            params={"q": query, "type": "track", "limit": 5},
        )
        return response.json()["tracks"]["items"]

    def play_track(self, track_uri):
        self._check_token()
        active_device = self.get_active_device()
        if active_device:
            response = requests.put(
                f"{self.base_url}/me/player/play",
                headers={"Authorization": f"Bearer {self.token}"},
                params={"device_id": active_device["id"]},
                json={"uris": [track_uri]},
            )
            print(response)
            return response.status_code in [204, 202]

    def get_active_device(self):
        self._check_token()
        response = requests.get(
            f"{self.base_url}/me/player/devices",
            headers={"Authorization": f"Bearer {self.token}"},
        )
        data = response.json()
        return next((device for device in data["devices"] if device["is_active"]), None)

    def pause(self):
        self._check_token()
        active_device = self.get_active_device()
        if active_device:
            response = requests.put(
                f"{self.base_url}/me/player/pause",
                headers={"Authorization": f"Bearer {self.token}"},
                params={"device_id": active_device["id"]},
            )
            return response.status_code in [204, 202]

    def set_volume(self, volume_percent):
        self._check_token()
        active_device = self.get_active_device()
        if active_device:
            response = requests.put(
                f"{self.base_url}/me/player/volume",
                headers={"Authorization": f"Bearer {self.token}"},
                params={
                    "volume_percent": int(volume_percent),
                    "device_id": active_device["id"],
                },
            )
            return response.status_code in [204, 202]

    def fade_out(self, duration_seconds=5):
        self._check_token()
        steps = 20
        step_duration = duration_seconds / steps

        for i in range(steps, -1, -1):
            volume = int((i / steps) * 100)
            self.set_volume(volume)
            time.sleep(step_duration)

    def fade_in(self, duration_seconds=5):
        self._check_token()
        steps = 20
        step_duration = duration_seconds / steps

        for i in range(steps + 1):
            volume = int((i / steps) * 100)
            self.set_volume(volume)
            time.sleep(step_duration)


if __name__ == "__main__":
    controller = SpotifyController()

    search_query = input("Enter song name to search: ")
    tracks = controller.search_track(search_query)

    for i, track in enumerate(tracks, 1):
        artists = ", ".join(artist["name"] for artist in track["artists"])
        print(f"{i}. {track['name']} by {artists}")

    choice = int(input("\nEnter number to play (0 to cancel): "))
    if 0 < choice <= len(tracks):
        if controller.play_track(tracks[choice - 1]["uri"]):
            print("Now playing!")
        else:
            print("Failed to play track")
