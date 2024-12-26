import os
import pandas as pd
from ytmusicapi import YTMusic
import spotipy
from spotipy import SpotifyClientCredentials


class SpotifyToYoutubeConverter:
    def __init__(self):
        # Initialize Spotify client
        auth_manager = SpotifyClientCredentials(
            client_id=os.getenv("SPOTIFY_CLIENT_ID"),
            client_secret=os.getenv("SPOTIFY_CLIENT_SECRET"),
        )
        self.spotify_client = spotipy.Spotify(auth_manager=auth_manager)

        # Initialize YouTube Music client
        self.ytmusic_client = YTMusic()

    def extract_spotify_id(self, spotify_url: str) -> str:
        """Extract Spotify track ID from full URL"""
        try:
            # Split the URL by 'si=' and take the part before it
            base_url = spotify_url.split("si=")[0]
            # Get the last part of the URL which is the track ID
            track_id = base_url.rstrip("?").split("/")[-1]
            return track_id
        except Exception as e:
            print(f"Error extracting Spotify ID from {spotify_url}: {str(e)}")
            return None

    def get_track_info(self, spotify_url: str) -> dict:
        """Get track information from Spotify URL"""
        try:
            track_id = self.extract_spotify_id(spotify_url)
            if not track_id:
                return None

            track_info = self.spotify_client.track(track_id)
            return {
                "name": track_info["name"],
                "artist": track_info["artists"][0]["name"],
            }
        except Exception as e:
            print(f"Error getting Spotify track info for {spotify_url}: {str(e)}")
            return None

    def search_youtube(self, track_name: str, artist_name: str) -> str:
        """Search for track on YouTube Music and return video ID"""
        try:
            search_query = f"{track_name} {artist_name}"
            results = self.ytmusic_client.search(search_query, filter="songs", limit=1)

            if results and len(results) > 0:
                return results[0]["videoId"]
            return None
        except Exception as e:
            print(f"Error searching YouTube for {track_name}: {str(e)}")
            return None

    def process_csv(self, input_file: str, output_file: str):
        """Process CSV file and update YouTube IDs"""
        try:
            # Read CSV file
            df = pd.read_csv(input_file)

            # Process each row
            for idx, row in df.iterrows():
                spotify_url = row["spotify_uri"]
                if not spotify_url or pd.isna(spotify_url):
                    continue

                # Get track info from Spotify
                track_info = self.get_track_info(spotify_url)
                if not track_info:
                    continue

                # Search on YouTube
                youtube_id = self.search_youtube(
                    track_info["name"], track_info["artist"]
                )
                if youtube_id:
                    df.at[idx, "youtube_id"] = youtube_id

                # Print progress
                if idx % 10 == 0:
                    print(f"Processed {idx} tracks...")

            # Save to new CSV file
            df.to_csv(output_file, index=False)
            print(f"Conversion completed. Results saved to {output_file}")

        except Exception as e:
            print(f"Error processing CSV: {str(e)}")
            raise


def main():
    # Check for required environment variables
    required_vars = ["SPOTIFY_CLIENT_ID", "SPOTIFY_CLIENT_SECRET"]
    missing_vars = [var for var in required_vars if var not in os.environ]
    if missing_vars:
        print(f"Missing required environment variables: {', '.join(missing_vars)}")
        print("Please set them before running the script.")
        return

    # Initialize converter
    converter = SpotifyToYoutubeConverter()

    # Process the file
    input_file = "../movie2.csv"  # Your input file
    output_file = "../movie2_yt.csv"

    converter.process_csv(input_file, output_file)


if __name__ == "__main__":
    main()
