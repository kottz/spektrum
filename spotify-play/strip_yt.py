import pandas as pd


def clean_spotify_uri(uri):
    """Remove the 'spotify:track:' prefix from URI"""
    return uri.replace("spotify:track:", "")


def clean_youtube_link(link):
    """Extract just the video ID from YouTube URL"""
    if pd.isna(link):
        return None
    return link.replace("https://www.youtube.com/watch?v=", "")


def cleanup_csv(input_file, output_file):
    # Read the CSV without treating first row as header
    df = pd.read_csv(
        input_file,
        names=[
            "index",
            "title",
            "artist",
            "spotify_uri",
            "color",
            "youtube_id",
            "youtube_link",
        ],
        header=None,
    )

    # Clean up Spotify URIs
    df["spotify_uri"] = df["spotify_uri"].apply(clean_spotify_uri)

    # Create new DataFrame with desired column order
    new_df = pd.DataFrame(
        {
            "index": df["index"],
            "title": df["title"],
            "artist": df["artist"],
            "color": df["color"],
            "spotify_id": df["spotify_uri"],
            "youtube_id": df["youtube_id"],
        }
    )

    # Write to new CSV file without index
    new_df.to_csv(output_file, index=False)
    print(f"Cleaned CSV saved to {output_file}")


if __name__ == "__main__":
    input_file = "spotify_tracks_with_youtube.csv"  # Your input file
    output_file = "spotify_tracks_with_youtube_stripped.csv"  # Output file name
    cleanup_csv(input_file, output_file)
