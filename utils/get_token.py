import http.server
import socketserver
import webbrowser
from urllib.parse import urlencode, parse_qs
import requests
import base64
import os
from dotenv import load_dotenv

load_dotenv()


class CallbackHandler(http.server.SimpleHTTPRequestHandler):
    def do_GET(self):
        query_components = parse_qs(self.path.split('?')[1])
        code = query_components['code'][0]

        # Exchange code for tokens
        client_id = os.getenv('SPOTIFY_CLIENT_ID')
        client_secret = os.getenv('SPOTIFY_CLIENT_SECRET')
        auth = base64.b64encode(
            f"{client_id}:{client_secret}".encode()).decode()

        response = requests.post(
            'https://accounts.spotify.com/api/token',
            headers={'Authorization': f'Basic {auth}'},
            data={
                'grant_type': 'authorization_code',
                'code': code,
                'redirect_uri': 'http://localhost:8888/callback'
            }
        )

        tokens = response.json()
        print("\nAdd this to your .env file:")
        print(f"SPOTIFY_REFRESH_TOKEN={tokens.get('refresh_token')}")

        self.send_response(200)
        self.end_headers()
        self.wfile.write(b"You can close this window now")
        raise KeyboardInterrupt()


def start_auth():
    client_id = os.getenv('SPOTIFY_CLIENT_ID')
    auth_params = {
        'client_id': client_id,
        'response_type': 'code',
        'redirect_uri': 'http://localhost:8888/callback',
        'scope': 'user-modify-playback-state user-read-playback-state'
    }

    auth_url = f"https://accounts.spotify.com/authorize?{
        urlencode(auth_params)}"

    with socketserver.TCPServer(("", 8888), CallbackHandler) as httpd:
        print("Opening browser for authorization...")
        webbrowser.open(auth_url)
        httpd.handle_request()


if __name__ == '__main__':
    start_auth()
