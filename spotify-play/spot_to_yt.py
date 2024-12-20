import os
import csv
from typing import List, Dict
import pandas as pd
from ytmusicapi import YTMusic
import spotipy
from spotipy import SpotifyClientCredentials

class SpotifyToYoutubeConverter:
    def __init__(self):
        # Initialize Spotify client
        auth_manager = SpotifyClientCredentials(
            client_id=os.getenv('SPOTIFY_CLIENT_ID'),
            client_secret=os.getenv('SPOTIFY_CLIENT_SECRET')
        )
        self.spotify_client = spotipy.Spotify(auth_manager=auth_manager)
        
        # Initialize YouTube Music client
        self.ytmusic_client = YTMusic()

    def get_track_info(self, spotify_uri: str) -> Dict:
        """Get track information from Spotify URI"""
        try:
            # Remove 'spotify:track:' prefix if present
            track_id = spotify_uri.split(':')[-1]
            track_info = self.spotify_client.track(track_id)
            return {
                'name': track_info['name'],
                'artist': track_info['artists'][0]['name']
            }
        except Exception as e:
            print(f"Error getting Spotify track info for {spotify_uri}: {str(e)}")
            return None

    def search_youtube(self, track_name: str, artist_name: str) -> str:
        """Search for track on YouTube Music and return video ID"""
        try:
            search_query = f"{track_name} {artist_name}"
            results = self.ytmusic_client.search(search_query, filter='songs', limit=1)
            
            if results and len(results) > 0:
                return results[0]['videoId']
            return None
        except Exception as e:
            print(f"Error searching YouTube for {track_name}: {str(e)}")
            return None

    def process_csv(self, input_file: str, output_file: str):
        """Process CSV file and add YouTube links"""
        try:
            # Read CSV file with no header, and assign column names
            df = pd.read_csv(input_file, header=0)
            
            # Add new column for YouTube IDs
            df['youtube_id'] = None
            df['youtube_link'] = None
            
            # Process each row
            for idx, row in df.iterrows():
                spotify_uri = row[3]  # Column 4 contains the Spotify URIs
                if not spotify_uri or pd.isna(spotify_uri):
                    continue
                    
                # Get track info from Spotify - we already have artist and track name
                track_name = row[1]  # Column 2 contains track names
                artist_name = row[2]  # Column 3 contains artist names
                
                # Search on YouTube
                youtube_id = self.search_youtube(track_name, artist_name)
                if youtube_id:
                    df.at[idx, 'youtube_id'] = youtube_id
                    df.at[idx, 'youtube_link'] = f"https://www.youtube.com/watch?v={youtube_id}"
                
                # Print progress
                if idx % 10 == 0:
                    print(f"Processed {idx} tracks...")

            # Save to new CSV file
            df.to_csv(output_file, index=False)
            print(f"Conversion completed. Results saved to {output_file}")
            
        except Exception as e:
            print(f"Error processing CSV: {str(e)}")
            raise  # Re-raise the exception to see the full error message

def main():
    # Check for required environment variables
    required_vars = ['SPOTIFY_CLIENT_ID', 'SPOTIFY_CLIENT_SECRET']
    missing_vars = [var for var in required_vars if var not in os.environ]
    if missing_vars:
        print(f"Missing required environment variables: {', '.join(missing_vars)}")
        print("Please set them before running the script.")
        return

    # Initialize converter
    converter = SpotifyToYoutubeConverter()
    
    # Process the file
    input_file = "/tmp/spot/spotify_tracks.csv"  # Your input file
    output_file = "spotify_tracks_with_youtube.csv"
    
    converter.process_csv(input_file, output_file)

if __name__ == "__main__":
    main()
