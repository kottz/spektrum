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
        self.answer = None


class GameLobby:
    def __init__(self, name):
        self.name = name
        self.players = {}  # name -> Player object
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
            del self.players[name]

    def get_player_list(self):
        return [{"name": player.name, "score": player.score} for player in self.players.values() if player.websocket]

    def select_round_colors(self, specified_colors=None):
        self.round_colors = []
        self.correct_colors = []

        if specified_colors:
            for color in specified_colors:
                color_obj = next(
                    (c for c in self.all_colors if c["name"].lower() == color.lower()), None)
                if color_obj:
                    self.round_colors.append(color_obj)
                    self.correct_colors.append(color_obj["name"])

        excluded_colors = set()
        for color in self.round_colors:
            if color["name"] in ["Yellow", "Gold", "Orange"]:
                excluded_colors.update(["Yellow", "Gold", "Orange"])
            elif color["name"] in ["Silver", "Gray"]:
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

    def start_new_round(self, specified_colors=None):
        self.select_round_colors(specified_colors)
        self.round_start_time = time.time()
        self.state = "question"
        for player in self.players.values():
            player.has_answered = False
            player.answer = None

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
    for player in lobby.players.values():
        if player.websocket:
            await send_game_state(player)


async def broadcast_answer_count():
    answered, total = lobby.get_answer_count()
    for player in lobby.players.values():
        if player.websocket:
            await player.websocket.send(json.dumps({
                "action": "update_answer_count",
                "answeredCount": answered,
                "totalPlayers": total
            }))


async def broadcast_player_answered(player_name):
    for player in lobby.players.values():
        if player.websocket:
            await player.websocket.send(json.dumps({
                "action": "player_answered",
                "playerName": player_name
            }))


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
                print(f"Correct color(s) for this round: {', '.join(lobby.correct_colors)}")
                print(f"All colors for this round: {', '.join(c['name'] for c in lobby.round_colors)}")
            await broadcast_game_state()
            await broadcast_answer_count()

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
