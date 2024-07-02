import asyncio
import websockets
import json
import argparse
import random
import aioconsole
import time


class Player:
    def __init__(self, name):
        self.name = name
        self.score = 0
        self.websocket = None
        self.has_answered = False


class GameLobby:
    def __init__(self, name):
        self.name = name
        self.players = {}  # name -> Player object
        self.colors = [
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
            {"name": "Brown", "rgb": "#A52A2A"},
            {"name": "Orange", "rgb": "#FFA500"},
            {"name": "Gray", "rgb": "#808080"},
        ]
        self.correct_colors = []
        self.state = "score"  # start with score state
        self.round_start_time = None
        self.round_duration = 50  # 50 seconds per round

    def add_player(self, name, websocket):
        if name not in self.players:
            self.players[name] = Player(name)
        self.players[name].websocket = websocket
        return self.players[name]

    def remove_player(self, name):
        if name in self.players:
            self.players[name].websocket = None

    def get_player_list(self):
        return [{"name": player.name, "score": player.score} for player in self.players.values() if player.websocket]

    def start_new_round(self, specified_colors=None):
        if specified_colors:
            self.correct_colors = [color.strip().capitalize()
                                   for color in specified_colors]
        else:
            self.correct_colors = [random.choice(self.colors)["name"]]
        self.round_start_time = time.time()
        self.state = "question"
        for player in self.players.values():
            player.has_answered = False

    def check_answer(self, player_name, color_name):
        if self.state != "question":
            return False, 0

        elapsed_time = time.time() - self.round_start_time
        if elapsed_time > self.round_duration:
            return False, 0

        # 100 points per second
        score = max(0, int(5000 - (elapsed_time * 100)))
        is_correct = color_name in self.correct_colors

        if is_correct:
            self.players[player_name].score += score

        self.players[player_name].has_answered = True

        # Print player's answer and current state
        print(f"Player {player_name} answered: {color_name}")
        self.print_answer_status()

        return is_correct, score

    def print_answer_status(self):
        answered = sum(1 for player in self.players.values()
                       if player.has_answered)
        total = len(self.players)
        print(f"Players answered: ({answered}/{total})")

    def toggle_state(self, specified_colors=None):
        if self.state == "score":
            self.start_new_round(specified_colors)
        else:
            self.state = "score"
        return self.state

    def all_players_answered(self):
        return all(player.has_answered for player in self.players.values())


lobby = None


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
                    if lobby.all_players_answered():
                        print(
                            "All players have answered. You can safely toggle to the next phase.")
    finally:
        if player:
            lobby.remove_player(player.name)
            print(f"Player {player.name} left the lobby")
            await broadcast_game_state()


async def send_game_state(player):
    await player.websocket.send(json.dumps({
        "action": "game_state",
        "state": lobby.state,
        "score": player.score,
        "colors": lobby.colors if lobby.state == "question" else None,
        "leaderboard": lobby.get_player_list() if lobby.state == "score" else None,
        "roundStartTime": lobby.round_start_time if lobby.state == "question" else None,
        "roundDuration": lobby.round_duration
    }))


async def broadcast_game_state():
    for player in lobby.players.values():
        if player.websocket:
            await send_game_state(player)


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
            await broadcast_game_state()

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Game Lobby WebSocket Server")
    parser.add_argument("--lobby", required=True, help="Name of the lobby")
    parser.add_argument("--port", type=int, default=8765,
                        help="Port to run the server on")
    args = parser.parse_args()

    lobby = GameLobby(args.lobby)
    print(f"Created lobby: {args.lobby}")

    start_server = websockets.serve(handle_client, "0.0.0.0", args.port)

    loop = asyncio.get_event_loop()
    loop.run_until_complete(start_server)
    print(f"WebSocket server started on 0.0.0.0:{args.port}")

    loop.create_task(handle_admin_input())
    loop.run_forever()
