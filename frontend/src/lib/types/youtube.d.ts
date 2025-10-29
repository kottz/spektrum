interface YT {
	Player: {
		new (
			elementId: string,
			config: {
				height?: string | number;
				width?: string | number;
				videoId?: string;
				playerVars?: {
					controls?: number;
					playsinline?: number;
					enablejsapi?: number;
				};
				events?: {
					onReady?: (event: YT.PlayerEvent) => void;
					onStateChange?: (event: YT.OnStateChangeEvent) => void;
					onError?: (event: YT.OnErrorEvent) => void;
				};
			}
		): YT.Player;
	};
	PlayerState: {
		UNSTARTED: -1;
		ENDED: 0;
		PLAYING: 1;
		PAUSED: 2;
		BUFFERING: 3;
		CUED: 5;
	};
}

declare namespace YT {
	interface Player {
		playVideo(): void;
		pauseVideo(): void;
		stopVideo(): void;
		cueVideoById(videoId: string): void;
		loadVideoById(videoId: string): void;
		getVideoData(): { video_id: string };
		destroy(): void;
		addEventListener(event: string, listener: (event: any) => void): void;
		removeEventListener(event: string, listener: (event: any) => void): void;
	}

	interface PlayerEvent {
		target: Player;
	}

	interface OnStateChangeEvent {
		target: Player;
		data: number;
	}

	interface OnErrorEvent {
		target: Player;
		data: number;
	}
}

declare const YT: YT;
