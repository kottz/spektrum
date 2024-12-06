import asyncio
import websockets
import json
import argparse
import random
import aioconsole
import time
import csv
import requests
import os
from dotenv import load_dotenv
import base64
from datetime import datetime, timedelta
import sys

load_dotenv()


class Player:
    def __init__(self, name):
        self.name = name
        self.score = 0
        self.websocket = None
        self.has_answered = False
        self.answer = None


class GameLobby:
    def __init__(self, name, songs):
        self.name = name
        self.players = {}
        self.all_colors = [
            {"name": "Red", "rgb": "#FF0000"},
            {"name": "Green", "rgb": "#00FF00"},
            {"name": "Blue", "rgb": "#0000FF"},
            {"name": "Yellow", "rgb": "#FFFF00"},
            {"name": "Purple", "rgb": "#800080"},
            {"name": "Gold", "rgb": "#FFD700"},
            {"name": "Silver", "rgb": "#C0C0C0"},
            {"name": "Pink", "rgb": "#FFC0CB"},
            {"name": "Black", "rgb": "#000000"},
            {"name": "White", "rgb": "#FFFFFF"},
            {"name": "Brown", "rgb": "#3D251E"},
            {"name": "Orange", "rgb": "#FFA500"},
            {"name": "Gray", "rgb": "#808080"},
        ]
        self.round_colors = []
        self.correct_colors = []
        self.state = "score"
        self.round_start_time = None
        self.round_duration = 50
        self.songs = songs
        self.used_songs = set()
        self.current_song = None

    def add_player(self, name, websocket):
        if name not in self.players:
            self.players[name] = Player(name)
        self.players[name].websocket = websocket
        return self.players[name]

    def remove_player(self, name):
        if name in self.players:
            del self.players[name]

    def get_player_list(self):
        return [{"name": player.name, "score": player.score} for player in self.players.values() if player.websocket]

    def select_round_colors(self, specified_colors=None):
        self.round_colors = []
        self.correct_colors = []

        if specified_colors:
            chosen_correct_colors = []
            for color in specified_colors:
                color_obj = next(
                    (c for c in self.all_colors if c["name"].lower() == color.lower()), None)
                if color_obj:
                    chosen_correct_colors.append(color_obj)
            if not chosen_correct_colors:
                print("No valid specified colors found.")
                return False
            self.round_colors.extend(chosen_correct_colors)
            self.correct_colors = [c["name"] for c in chosen_correct_colors]

            excluded_colors = set()
            all_correct_color_names = [c["name"]
                                       for c in chosen_correct_colors]

            if any(cc in ["Yellow", "Gold", "Orange"] for cc in all_correct_color_names):
                excluded_colors.update(["Yellow", "Gold", "Orange"])
            if any(cc in ["Silver", "Gray"] for cc in all_correct_color_names):
                excluded_colors.update(["Silver", "Gray"])

            available_colors = [c for c in self.all_colors if c["name"]
                                not in excluded_colors and c not in self.round_colors]

            while len(self.round_colors) < 6 and available_colors:
                chosen_color = random.choice(available_colors)
                self.round_colors.append(chosen_color)
                available_colors.remove(chosen_color)
                if chosen_color["name"] in ["Yellow", "Gold", "Orange"]:
                    available_colors = [c for c in available_colors if c["name"] not in [
                        "Yellow", "Gold", "Orange"]]
                elif chosen_color["name"] in ["Silver", "Gray"]:
                    available_colors = [
                        c for c in available_colors if c["name"] not in ["Silver", "Gray"]]

            random.shuffle(self.round_colors)
            return True
        else:
            available_songs = [
                s for s in self.songs if s['uri'] not in self.used_songs]
            if not available_songs:
                print("No more songs available. The game ends here.")
                return False

            chosen_song = random.choice(available_songs)
            chosen_correct_color_names = chosen_song['colors']

            chosen_correct_colors = []
            for c_name in chosen_correct_color_names:
                color_obj = next(
                    (c for c in self.all_colors if c["name"].lower() == c_name.lower()), None)
                if color_obj:
                    chosen_correct_colors.append(color_obj)
                else:
                    print(f"Color {c_name} not found in all_colors list.")
            if not chosen_correct_colors:
                return False

            self.current_song = chosen_song
            self.round_colors.extend(chosen_correct_colors)
            self.correct_colors = [c["name"] for c in chosen_correct_colors]

            excluded_colors = set()
            if any(cc["name"] in ["Yellow", "Gold", "Orange"] for cc in chosen_correct_colors):
                excluded_colors.update(["Yellow", "Gold", "Orange"])
            if any(cc["name"] in ["Silver", "Gray"] for cc in chosen_correct_colors):
                excluded_colors.update(["Silver", "Gray"])

            available_colors = [c for c in self.all_colors if c["name"]
                                not in excluded_colors and c not in self.round_colors]

            while len(self.round_colors) < 6 and available_colors:
                chosen_color_obj = random.choice(available_colors)
                self.round_colors.append(chosen_color_obj)
                available_colors.remove(chosen_color_obj)
                if chosen_color_obj["name"] in ["Yellow", "Gold", "Orange"]:
                    available_colors = [c for c in available_colors if c["name"] not in [
                        "Yellow", "Gold", "Orange"]]
                elif chosen_color_obj["name"] in ["Silver", "Gray"]:
                    available_colors = [
                        c for c in available_colors if c["name"] not in ["Silver", "Gray"]]

            random.shuffle(self.round_colors)
            return True

    def start_new_round(self, specified_colors=None):
        success = self.select_round_colors(specified_colors)
        if not success:
            return
        self.round_start_time = time.time()
        self.state = "question"
        for player in self.players.values():
            player.has_answered = False
            player.answer = None

    def end_round(self):
        if self.current_song:
            self.used_songs.add(self.current_song['uri'])
        self.current_song = None
        self.state = "score"

    def check_answer(self, player_name, color_name):
        if self.state != "question":
            return False, 0

        player = self.players[player_name]
        if player.has_answered:
            return player.answer in self.correct_colors, player.score

        elapsed_time = time.time() - self.round_start_time
        if elapsed_time > self.round_duration:
            return False, 0

        score = max(0, int(5000 - (elapsed_time * 100)))
        is_correct = color_name in self.correct_colors

        if is_correct:
            player.score += score

        player.has_answered = True
        player.answer = color_name

        print(f"Player {player_name} answered: {color_name}")
        self.print_answer_status()

        return is_correct, score

    def print_answer_status(self):
        answered = sum(1 for player in self.players.values()
                       if player.has_answered)
        total = len(self.players)
        print(f"Players answered: ({answered}/{total})")

    def get_answer_count(self):
        answered = sum(1 for player in self.players.values()
                       if player.has_answered)
        total = len(self.players)
        return answered, total

    def toggle_state(self, specified_colors=None):
        if self.state == "score":
            self.start_new_round(specified_colors)
        else:
            self.end_round()
        return self.state

    def all_players_answered(self):
        return all(player.has_answered for player in self.players.values())


async def handle_client(websocket, path):
    player = None
    try:
        async for message in websocket:
            data = json.loads(message)
            if data['action'] == 'join':
                name = data['name']
                player = lobby.add_player(name, websocket)
                print(f"Player {name} joined the lobby")
                await send_game_state(player)
                await broadcast_answer_count()
            elif data['action'] == 'select_color':
                if player and lobby.state == "question":
                    color_name = data['color']
                    is_correct, score = lobby.check_answer(
                        player.name, color_name)
                    await websocket.send(json.dumps({
                        "action": "color_result",
                        "correct": is_correct,
                        "score": score,
                        "totalScore": player.score
                    }))
                    await broadcast_player_answered(player.name)
                    await broadcast_answer_count()
                    if lobby.all_players_answered():
                        print(
                            "All players have answered. You can safely toggle to the next phase.")
    finally:
        if player:
            lobby.remove_player(player.name)
            print(f"Player {player.name} left the lobby")
            await broadcast_game_state()
            await broadcast_answer_count()


async def send_game_state(player):
    answered, total = lobby.get_answer_count()
    await player.websocket.send(json.dumps({
        "action": "game_state",
        "state": lobby.state,
        "score": player.score,
        "colors": lobby.round_colors if lobby.state == "question" else None,
        "leaderboard": lobby.get_player_list() if lobby.state == "score" else None,
        "roundStartTime": lobby.round_start_time if lobby.state == "question" else None,
        "roundDuration": lobby.round_duration,
        "hasAnswered": player.has_answered,
        "answer": player.answer,
        "answeredCount": answered,
        "totalPlayers": total
    }))


async def broadcast_game_state():
    for p in lobby.players.values():
        if p.websocket:
            await send_game_state(p)


async def broadcast_answer_count():
    answered, total = lobby.get_answer_count()
    for p in lobby.players.values():
        if p.websocket:
            await p.websocket.send(json.dumps({
                "action": "update_answer_count",
                "answeredCount": answered,
                "totalPlayers": total
            }))


async def broadcast_player_answered(player_name):
    for p in lobby.players.values():
        if p.websocket:
            await p.websocket.send(json.dumps({
                "action": "player_answered",
                "playerName": player_name
            }))


def load_songs_from_csv(filepath):
    songs = []
    with open(filepath, 'r', encoding='utf-8') as f:
        reader = csv.DictReader(
            f, fieldnames=['id', 'song_name', 'artist', 'uri', 'colors'])
        for row in reader:
            colors = [color.strip().capitalize()
                      for color in row['colors'].split(';')] if row['colors'] else []
            song_entry = {
                'id': int(row['id']),
                'song_name': row['song_name'].strip(),
                'artist': row['artist'].strip(),
                'uri': row['uri'].strip(),
                'colors': colors
            }
            songs.append(song_entry)
    return songs


def save_songs_to_csv(songs, filepath):
    with open(filepath, 'w', newline='', encoding='utf-8') as f:
        writer = csv.DictWriter(
            f, fieldnames=['id', 'song_name', 'artist', 'uri', 'colors'])
        for song in songs:
            song_row = song.copy()
            song_row['colors'] = ';'.join(song['colors'])
            writer.writerow(song_row)


class SpotifyController:
    def __init__(self):
        self.base_url = 'https://api.spotify.com/v1'
        self.client_id = os.getenv('SPOTIFY_CLIENT_ID')
        self.client_secret = os.getenv('SPOTIFY_CLIENT_SECRET')
        self.refresh_token = os.getenv('SPOTIFY_REFRESH_TOKEN')
        self.token = None
        self.token_expiry = None
        self._get_access_token()

    def _get_access_token(self):
        auth = base64.b64encode(
            f"{self.client_id}:{self.client_secret}".encode()).decode()
        response = requests.post(
            'https://accounts.spotify.com/api/token',
            headers={'Authorization': f'Basic {auth}'},
            data={
                'grant_type': 'refresh_token',
                'refresh_token': self.refresh_token
            }
        )
        if response.status_code == 200:
            data = response.json()
            self.token = data.get('access_token', None)
            expires_in = data.get('expires_in', 0)
            self.token_expiry = datetime.now() + timedelta(seconds=expires_in)
        else:
            print("Failed to get Spotify access token.")
            sys.exit(1)

    def _check_token(self):
        if not self.token or datetime.now() >= self.token_expiry:
            self._get_access_token()

    def search_track(self, query):
        self._check_token()
        response = requests.get(
            f'{self.base_url}/search',
            headers={'Authorization': f'Bearer {self.token}'},
            params={'q': query, 'type': 'track', 'limit': 5}
        )
        data = response.json()
        return data.get('tracks', {}).get('items', [])

    def get_active_device(self):
        self._check_token()
        response = requests.get(
            f'{self.base_url}/me/player/devices',
            headers={'Authorization': f'Bearer {self.token}'}
        )
        if response.status_code != 200:
            print("Error retrieving devices from Spotify.")
            return None
        data = response.json()
        devices = data.get('devices', [])
        if not devices:
            print("No devices found. Please open Spotify on a device.")
            return None
        active_device = next(
            (device for device in devices if device.get('is_active')), None)
        return active_device

    def play_track(self, track_uri):
        self._check_token()
        active_device = self.get_active_device()
        if not active_device:
            print("No active Spotify device found. Cannot play track.")
            return False
        response = requests.put(
            f'{self.base_url}/me/player/play',
            headers={'Authorization': f'Bearer {self.token}'},
            params={'device_id': active_device['id']},
            json={'uris': [track_uri]}
        )
        if response.status_code not in [204, 202]:
            print(f"Failed to play track on Spotify. Status code: {
                  response.status_code}")
            return False
        return True

    def pause(self):
        self._check_token()
        active_device = self.get_active_device()
        if not active_device:
            print("No active device to pause.")
            return False
        response = requests.put(
            f'{self.base_url}/me/player/pause',
            headers={'Authorization': f'Bearer {self.token}'},
            params={'device_id': active_device['id']}
        )
        return response.status_code in [204, 202]

    def set_volume(self, volume_percent):
        self._check_token()
        active_device = self.get_active_device()
        if not active_device:
            print("No active device to set volume.")
            return False
        response = requests.put(
            f'{self.base_url}/me/player/volume',
            headers={'Authorization': f'Bearer {self.token}'},
            params={
                'volume_percent': int(volume_percent),
                'device_id': active_device['id']
            }
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


async def handle_admin_input():
    while True:
        command = await aioconsole.ainput("Enter 'toggle' or 'toggle color1,color2,...' to switch game state: ")
        if command.lower().startswith('toggle'):
            parts = command.split(None, 1)
            if len(parts) > 1:
                colors = parts[1].split(',')
                new_state = lobby.toggle_state(colors)
            else:
                new_state = lobby.toggle_state()

            print(f"Game state changed to: {new_state}")
            if new_state == "question":
                print(f"Correct color(s) for this round: {
                      ', '.join(lobby.correct_colors)}")
                print(f"All colors for this round: {
                      ', '.join(c['name'] for c in lobby.round_colors)}")
                if lobby.current_song:
                    print(f"Selected track: {lobby.current_song['song_name']} by {
                          lobby.current_song['artist']} - {lobby.current_song['uri']}")
                if spotify_enabled:
                    success = controller.play_track(lobby.current_song['uri'])
                    if not success:
                        print(
                            "Could not start playback. Please check your Spotify setup.")
            elif new_state == "score" and spotify_enabled:
                controller.pause()

            await broadcast_game_state()
            await broadcast_answer_count()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Game Lobby WebSocket Server")
    parser.add_argument("--lobby", required=True, help="Name of the lobby")
    parser.add_argument("--port", type=int, default=8765,
                        help="Port to run the server on")
    parser.add_argument("--songs_csv", required=True,
                        help="Path to the songs CSV file")
    parser.add_argument("--no-spotify", action='store_true',
                        help="Disable Spotify integration")
    args = parser.parse_args()

    songs = load_songs_from_csv(args.songs_csv)
    lobby = GameLobby(args.lobby, songs)
    print(f"Created lobby: {args.lobby}")

    spotify_enabled = not args.no_spotify
    if spotify_enabled:
        controller = SpotifyController()
        # Test Spotify connectivity and device availability immediately
        active_device = controller.get_active_device()
        if not active_device:
            print(
                "No active Spotify device found at startup. Please ensure a Spotify device is active.")
            sys.exit(1)
        else:
            print(f"Found active Spotify device: {
                  active_device.get('name', 'Unknown Device')}")
    else:
        controller = None
        print("Spotify integration disabled. Running without automatic playback.")

    # Wrap the original toggle_state to handle Spotify conditionally
    original_toggle_state = lobby.toggle_state

    def toggle_state_wrapper(specified_colors=None):
        old_state = lobby.state
        new_state = original_toggle_state(specified_colors)
        # Only attempt playback if spotify is enabled
        if spotify_enabled:
            if new_state == "question" and hasattr(lobby, 'current_song') and lobby.current_song:
                print(f"Attempting to play track: {lobby.current_song['song_name']} by {
                      lobby.current_song['artist']}")
                success = controller.play_track(lobby.current_song['uri'])
                if not success:
                    print("Could not start playback. Please check your Spotify setup.")
            elif new_state == "score":
                controller.pause()
        return new_state
    lobby.toggle_state = toggle_state_wrapper

    start_server = websockets.serve(handle_client, "0.0.0.0", args.port)
    loop = asyncio.get_event_loop()
    loop.run_until_complete(start_server)
    print(f"WebSocket server started on 0.0.0.0:{args.port}")

    loop.create_task(handle_admin_input())
    loop.run_forever()
