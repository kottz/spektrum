// types.js
/**
 * @typedef {Object} Player
 * @property {string} name
 * @property {number} score
 */

/**
 * @typedef {Object} Color
 * @property {string} name
 * @property {string} rgb
 */

/**
 * @typedef {Object} GameState
 * @property {string} phase
 * @property {Color[]} colors
 * @property {Player[]} scoreboard
 */

/**
 * @typedef {Object} AnsweredPlayer
 * @property {string} playerName
 * @property {boolean} correct
 */

/**
 * @typedef {Object} Song
 * @property {string} song_name
 * @property {string} artist
 * @property {string} youtube_id
 */

// config.js
const CONFIG = {
  ROUND_DURATION: 60,
  NOTIFICATION_DURATION: 3000,
  TIMER_INTERVAL: 100,
  WS_PROTOCOLS: {
    HTTP: 'ws:',
    HTTPS: 'wss:'
  }
};

// gameState.js
class GameState {
  constructor() {
    /** @type {WebSocket|null} */
    this.socket = null;
    /** @type {string} */
    this.playerName = '';
    /** @type {string|null} */
    this.currentLobbyId = null;
    /** @type {string|null} */
    this.currentAdminId = null;
    /** @type {boolean} */
    this.gameStarted = false;
    /** @type {boolean} */
    this.isAdmin = false;
    /** @type {Color[]} */
    this.colors = [];
    /** @type {string|null} */
    this.playerAnswer = null;
    /** @type {boolean} */
    this.hasAnswered = false;
    /** @type {number} */
    this.playerScore = 0;
    /** @type {number} */
    this.totalPlayers = 0;
    /** @type {AnsweredPlayer[]} */
    this.answeredPlayers = [];
    /** @type {YT.Player|null} */
    this.youtubePlayer = null;
    /** @type {string|null} */
    this.nextYoutubeId = null;
    /** @type {boolean} */
    this.isLoadingNextSong = false;
  }

  reset() {
    this.playerScore = 0;
    this.hasAnswered = false;
    this.playerAnswer = null;
    this.answeredPlayers = [];
    this.colors = [];
    this.gameStarted = false;
  }
}

// youtubeManager.js
class YoutubeManager {
  /**
   * @param {GameState} gameState
   */
  constructor(gameState) {
    this.gameState = gameState;
    this.initializeAPI();
  }

  cleanup() {
    if (this.gameState.youtubePlayer) {
      this.gameState.youtubePlayer.destroy();
      this.gameState.youtubePlayer = null;
    }
  }

  initializeAPI() {
    if (document.querySelector('script[src*="youtube.com/iframe_api"]')) return;

    const tag = document.createElement('script');
    tag.src = "https://www.youtube.com/iframe_api";
    const firstScriptTag = document.getElementsByTagName('script')[0];
    firstScriptTag.parentNode.insertBefore(tag, firstScriptTag);
  }

  /**
   * @param {string} containerId
   */
  initializePlayer(containerId) {
    this.gameState.youtubePlayer = new YT.Player(containerId, {
      height: '100%',
      width: '100%',
      playerVars: {
        controls: 0,
        playsinline: 1,
        enablejsapi: 1
      },
      events: {
        onReady: this.onPlayerReady.bind(this),
        onStateChange: this.onPlayerStateChange.bind(this)
      }
    });
  }

  /**
     * Verifies if the correct video is loaded and updates if necessary
     * @param {string} expectedVideoId The video ID that should be playing
     * @returns {Promise<boolean>} Returns true if video was correct or successfully updated
     */
  async verifyAndUpdateVideo(expectedVideoId) {
    if (!this.gameState.youtubePlayer || !expectedVideoId) return false;

    try {
      const currentVideoId = this.gameState.youtubePlayer.getVideoData()?.video_id;

      if (currentVideoId !== expectedVideoId) {
        console.log('Detected video mismatch, updating player...', {
          current: currentVideoId,
          expected: expectedVideoId
        });

        return new Promise((resolve) => {
          // Set up one-time event listener for when video is ready
          const onStateChange = (event) => {
            if (event.data === YT.PlayerState.CUED) {
              this.gameState.youtubePlayer.removeEventListener('onStateChange', onStateChange);
              resolve(true);
            }
          };

          this.gameState.youtubePlayer.addEventListener('onStateChange', onStateChange);
          this.gameState.youtubePlayer.loadVideoById(expectedVideoId);
        });
      }

      return true;
    } catch (error) {
      console.error('Error verifying YouTube video:', error);
      return false;
    }
  }

  /**
   * @param {YT.PlayerEvent} event
   */
  onPlayerReady(event) {
    if (this.gameState.nextYoutubeId) {
      event.target.cueVideoById(this.gameState.nextYoutubeId);
    }
  }

  /**
   * @param {YT.OnStateChangeEvent} event
   */
  onPlayerStateChange(event) {
    if (event.data === YT.PlayerState.CUED) {
      this.gameState.isLoadingNextSong = false;
      this.updateButtonStates();
    }
  }

  updateButtonStates() {
    if (!this.gameState.isAdmin) return;

    const roundButton = document.querySelector('button[onclick="game.toggleRound()"]');
    if (roundButton) {
      const phase = document.getElementById("gameState").textContent;
      if (phase.includes("score")) {
        roundButton.textContent = this.gameState.isLoadingNextSong ? "Loading..." : "Start Round";
        roundButton.disabled = this.gameState.isLoadingNextSong;
      } else {
        roundButton.textContent = "End Round";
        roundButton.disabled = false;
      }
    }
  }

  /**
   * @param {string} youtubeId 
   */
  loadVideo(youtubeId) {
    if (this.gameState.youtubePlayer && this.gameState.youtubePlayer.cueVideoById) {
      this.gameState.youtubePlayer.cueVideoById(youtubeId);
    }
  }

  handleVideoControls(phase) {
    if (!this.gameState.youtubePlayer) return;

    if (phase === "question") {
      this.gameState.youtubePlayer.playVideo();
    } else if (phase === "lobby" || phase === "game_over") {
      this.gameState.youtubePlayer.stopVideo();
    }
  }
}

// websocketManager.js
class WebSocketManager {
  /**
   * @param {GameState} gameState
   * @param {UIManager} uiManager
   */
  constructor(gameState, uiManager) {
    this.gameState = gameState;
    this.uiManager = uiManager;
  }

  /**
   * @param {string} lobbyId
   * @param {string} name
   */
  connect(lobbyId, name) {
    this.closeConnection();

    const wsProtocol = window.location.protocol === "https:" ?
      CONFIG.WS_PROTOCOLS.HTTPS :
      CONFIG.WS_PROTOCOLS.HTTP;
    const wsUrl = `${wsProtocol}//${window.location.host}/ws`;

    this.gameState.socket = new WebSocket(wsUrl);
    this.setupSocketHandlers(lobbyId, name);
  }

  /**
   * @param {string} lobbyId
   * @param {string} name
   */
  setupSocketHandlers(lobbyId, name) {
    if (!this.gameState.socket) return;

    this.gameState.socket.onopen = () => {
      const joinMsg = {
        type: "JoinLobby",
        lobby_id: lobbyId,
        admin_id: this.gameState.isAdmin ? this.gameState.currentAdminId : null,
        name: this.gameState.isAdmin ? "Admin" : name
      };
      this.sendMessage(joinMsg);
    };

    this.gameState.socket.onmessage = this.handleServerMessage.bind(this);
    this.gameState.socket.onclose = () => { };
    this.gameState.socket.onerror = (error) => {
      console.error("WebSocket error:", error);
      this.uiManager.showNotification("Connection error", true);
    };
  }

  closeConnection() {
    if (this.gameState.socket) {
      this.gameState.socket.close();
      this.gameState.socket = null;
    }
  }

  /**
   * @param {any} message
   */
  sendMessage(message) {
    if (this.gameState.socket?.readyState === WebSocket.OPEN) {
      console.log("sending:", message);
      this.gameState.socket.send(JSON.stringify(message));
    } else {
      this.uiManager.showNotification("Connection not available", true);
    }
  }

  /**
   * @param {MessageEvent} event 
   */
  async handleServerMessage(event) {
    let data;
    try {
      data = JSON.parse(event.data);
    } catch (e) {
      console.error("Invalid message from server:", e);
      return;
    }
    console.log("received:", data);

    switch (data.type) {
      case "JoinedLobby":
        this.uiManager.handleJoinedLobby(data);
        break;
      case "InitialPlayerList":
        this.uiManager.updateInitialPlayerList(data.players);
        break;
      case "PlayerJoined":
        this.uiManager.handlePlayerJoined(data);
        break;
      case "PlayerLeft":
        this.uiManager.handlePlayerLeft(data);
        break;
      case "PlayerAnswered":
        this.uiManager.handlePlayerAnswered(data);
        break;
      case "StateChanged":
        await this.uiManager.handleStateChanged(data);
        break;
      case "GameOver":
        this.uiManager.handleGameOver(data);
        break;
      case "GameClosed":
        this.uiManager.handleGameClosed(data);
        break;
      case "AdminInfo":
        this.uiManager.handleAdminInfo(data);
        break;
      case "AdminNextSongs":
        this.uiManager.handleAdminNextSongs(data);
        break;
      case "Error":
        this.uiManager.showNotification(data.message, true);
        break;
      default:
        console.warn("Unhandled server message type:", data.type);
    }
  }
}

// uiManager.js
class UIManager {
  /**
     * @param {GameState} gameState
     * @param {YoutubeManager} ytManager
     * @param {GameController} gameController
     */
  constructor(gameState, ytManager, gameController) {
    this.gameState = gameState;
    this.ytManager = ytManager;
    this.gameController = gameController;
    this.timerInterval = null;
  }

  /**
   * @param {string} message
   * @param {boolean} isError
   */
  showNotification(message, isError = false) {
    const notification = document.createElement("div");
    notification.className = `notification ${isError ? "error" : "info"}`;
    notification.textContent = message;
    document.body.appendChild(notification);

    setTimeout(() => notification.remove(), CONFIG.NOTIFICATION_DURATION);
  }

  /**
   * @param {number} durationMs
   */
  startTimer(durationMs) {
    this.stopTimer();
    let timeLeftMs = durationMs;
    const timerElem = document.getElementById("timer");

    this.timerInterval = setInterval(() => {
      timeLeftMs -= CONFIG.TIMER_INTERVAL;
      if (timeLeftMs <= 0) {
        this.stopTimer();
        timerElem.textContent = "Time's up!";
      } else {
        const seconds = (timeLeftMs / 1000).toFixed(1);
        timerElem.textContent = `Time remaining: ${seconds}s`;
      }
    }, CONFIG.TIMER_INTERVAL);
  }

  stopTimer() {
    if (this.timerInterval) {
      clearInterval(this.timerInterval);
      this.timerInterval = null;
      const timerElem = document.getElementById("timer");
      timerElem.textContent = "Answer Received!";
    }
  }

  /**
     * @param {string[]} lobbyIds
     */
  updateLobbyList(lobbyIds) {
    const lobbyList = document.getElementById("lobbyList");
    lobbyList.innerHTML = "";

    if (lobbyIds.length === 0) {
      lobbyList.innerHTML = "<p>No active lobbies. Create one to get started!</p>";
      return;
    }

    // Store reference to the game controller
    const gameController = this.gameController;

    lobbyIds.forEach((id) => {
      const lobbyElement = document.createElement("div");
      lobbyElement.className = "lobby-item";
      lobbyElement.innerHTML = `
        <div class="lobby-info">
          <span>Lobby: ${id}</span>
        </div>
        <button class="join-lobby-btn" data-lobby-id="${id}">Join</button>
      `;

      // Add click handler for the join button using the stored reference
      const joinButton = lobbyElement.querySelector('.join-lobby-btn');
      joinButton.addEventListener('click', () => gameController.showJoinForm(id));

      lobbyList.appendChild(lobbyElement);
    });
  }

  /**
   * @param {Player[]} players 
   */
  updateLeaderboard(players) {
    const leaderboard = document.getElementById("leaderboard");
    leaderboard.innerHTML = "<h2>Leaderboard</h2>";

    const sortedPlayers = [...players].sort((a, b) => b.score - a.score);
    const maxScore = sortedPlayers.length > 0 ? sortedPlayers[0].score : 0;

    sortedPlayers.forEach((player, index) => {
      const playerItem = document.createElement("div");
      playerItem.className = "player-item";

      const playerInfo = document.createElement("div");
      playerInfo.className = "player-info";
      playerInfo.innerHTML = `
        <span>${index + 1}. ${player.name}</span>
        <span>${player.score} points</span>
      `;

      const progressBar = document.createElement("div");
      progressBar.className = "progress-bar";

      const progress = document.createElement("div");
      progress.className = "progress";
      const width = maxScore > 0 ? (player.score / maxScore) * 100 : 0;
      progress.style.width = `${width}%`;

      progressBar.appendChild(progress);
      playerItem.appendChild(playerInfo);
      playerItem.appendChild(progressBar);
      leaderboard.appendChild(playerItem);
    });
  }

  /**
   * @param {Color[]} colors
   */
  createColorButtons(colors) {
    const colorButtons = document.getElementById("colorButtons");
    colorButtons.innerHTML = "";

    colors.forEach((color) => {
      const button = document.createElement("button");
      button.className = "color-button";

      if (color.name.toLowerCase() === "gold") {
        button.classList.add("metallic-gold");
      } else if (color.name.toLowerCase() === "silver") {
        button.classList.add("metallic-silver");
      } else {
        button.style.backgroundColor = color.rgb;
      }

      // Use the gameController reference instead of window.game
      button.addEventListener('click', () => this.gameController.selectColor(color.name));
      button.title = color.name;

      if (this.gameState.hasAnswered && this.gameState.playerAnswer === color.name) {
        button.style.border = "3px solid white";
      }
      if (this.gameState.hasAnswered) {
        button.disabled = true;
      }

      colorButtons.appendChild(button);
    });
  }

  /**
     * @param {Object} data
     * @param {number} data.round_duration
     * @param {Player[]} data.players
     */
  handleJoinedLobby(data) {
    CONFIG.ROUND_DURATION = data.round_duration;
    this.updateLeaderboard(data.players);
    document.getElementById("joinForm").style.display = "none";
    document.getElementById("lobbyInfo").style.display = "block";
  }

  /**
   * @param {Player[]} players 
   */
  updateInitialPlayerList(players) {
    this.gameState.totalPlayers = players.length;
    this.updateLeaderboard(Array.isArray(players.players) ? players.players : players.map(([name, score]) => ({ name, score })));
  }

  /**
   * @param {Object} data
   * @param {string} data.player_name
   * @param {number} data.current_score
   */
  handlePlayerJoined(data) {
    this.gameState.totalPlayers += 1;
    this.showNotification(`${data.player_name} joined the game`);
  }

  /**
   * @param {Object} data
   * @param {string} data.name
   */
  handlePlayerLeft(data) {
    this.gameState.totalPlayers = Math.max(0, this.gameState.totalPlayers - 1);
    this.showNotification(`${data.name} left the game`);
  }

  /**
   * @param {Object} data
   * @param {string} data.name
   * @param {boolean} data.correct
   * @param {number} data.new_score
   */
  handlePlayerAnswered(data) {
    if (data.name === this.gameState.playerName) {
      this.gameState.playerScore = data.new_score;
      this.updateYourScore(data.new_score);
    }
    this.gameState.answeredPlayers.push({
      playerName: data.name,
      correct: data.correct
    });
    this.updateAnswerStatus();
  }

  /**
   * @param {Object} data
   * @param {string} data.phase
   * @param {Color[]} data.colors
   * @param {Array<[string, number]>} data.scoreboard
   */
  async handleStateChanged(data) {
    document.getElementById("gameState").textContent = `Current Phase: ${data.phase}`;
    this.stopTimer();

    // Update game started state
    this.gameState.gameStarted = data.phase !== "lobby";
    this.updateGameControls(data.phase);

    const currentPhaseElement = document.getElementById('currentPhase');
    if (currentPhaseElement) {
      currentPhaseElement.textContent = data.phase;
    }

    switch (data.phase) {
      case "lobby":
        this.handleLobbyPhase();
        break;
      case "score":
        this.handleScorePhase(data.scoreboard);
        break;
      case "question":
        await this.handleQuestionPhase(data.colors);
        break;
    }
  }
  /**
   * @param {Object} data
   * @param {Array<[string, number]>} data.scores
   * @param {string} data.reason
   */
  handleGameOver(data) {
    this.stopTimer();
    this.showNotification(`Game Over: ${data.reason}`, true);

    document.getElementById("colorButtons").style.display = "none";
    document.getElementById("answerStatusContainer").style.display = "none";
    document.getElementById("roundResult").textContent = "";
    document.getElementById("leaderboard").style.display = "block";

    const formattedScores = data.scores.map(([name, score]) => ({
      name,
      score
    }));
    this.updateLeaderboard(formattedScores);

    if (this.gameState.isAdmin) {
      this.handleAdminGameOver();
    }

    this.gameState.gameStarted = false;
    document.getElementById("gameState").textContent = "Game Over";
  }

  /**
   * @param {Object} data
   * @param {string} data.reason
   */
  handleGameClosed(data) {
    this.showNotification(`Game Closed: ${data.reason}`, true);
    this.resetGameState();
    document.getElementById("lobbyInfo").style.display = "none";
    document.getElementById("adminControls").style.display = "none";
    document.getElementById("lobbySelection").style.display = "block";
  }

  /**
   * @param {Object} data
   * @param {string} data.current_song_name
   * @param {string} data.current_song_artist
   * @param {string} data.current_song_youtube_id
   */
  handleAdminInfo(data) {
    if (!this.gameState.isAdmin) return;

    const songEl = document.getElementById("currentSong");
    songEl.textContent = `${data.current_song_name} by ${data.current_song_artist}`;

    this.gameState.nextYoutubeId = data.current_song_youtube_id;
    const currentPhase = document.getElementById("gameState").textContent;

    if (currentPhase.includes("score")) {
      this.ytManager.loadVideo(data.current_song_youtube_id);
    }
  }

  /**
   * @param {Object} data
   * @param {Song[]} data.upcoming_songs
   */
  handleAdminNextSongs(data) {
    if (!this.gameState.isAdmin || !data.upcoming_songs?.length) return;

    const nextSong = data.upcoming_songs[0];
    this.gameState.nextYoutubeId = nextSong.youtube_id;
    this.gameState.isLoadingNextSong = true;

    this.updateUpcomingSongsList(data.upcoming_songs);

    const currentPhase = document.getElementById("gameState").textContent;
    if (currentPhase.includes("score")) {
      this.ytManager.loadVideo(nextSong.youtube_id);
    }
  }

  /**
   * @param {Song[]} songs
   */
  updateUpcomingSongsList(songs) {
    const songsList = document.getElementById("songsList");
    songsList.innerHTML = "";

    songs.forEach((song, index) => {
      const songElement = document.createElement("div");
      songElement.className = "upcoming-song";
      songElement.innerHTML = `
        <span class="song-number">${index + 1}.</span>
        <span class="song-info">${song.song_name} - ${song.artist}</span>
      `;
      songsList.appendChild(songElement);
    });
  }

  updateAnswerStatus() {
    const counterElement = document.getElementById("answerCounter");
    counterElement.textContent = `${this.gameState.answeredPlayers.length}/${this.gameState.totalPlayers}`;

    const answeredPlayersElement = document.getElementById("answeredPlayers");
    answeredPlayersElement.innerHTML = "";

    this.gameState.answeredPlayers.forEach((p) => {
      const playerSpan = document.createElement("span");
      playerSpan.className = "answered-player";
      playerSpan.style.backgroundColor = p.correct ? "#35cf0e" : "#cf0e22";
      playerSpan.textContent = p.playerName;
      answeredPlayersElement.appendChild(playerSpan);
    });
  }

  /**
   * @param {number} score
   */
  updateYourScore(score) {
    document.getElementById("yourScore").textContent = score.toString();
  }

  resetGameState() {
    this.gameState.reset();
    document.getElementById("yourScore").textContent = "0";
    document.getElementById("leaderboard").innerHTML = "";
    document.getElementById("roundResult").textContent = "";
    document.getElementById("colorButtons").innerHTML = "";
    document.getElementById("answerStatusContainer").style.display = "none";
  }

  handleLobbyPhase() {
    document.getElementById("colorButtons").style.display = "none";
    document.getElementById("leaderboard").style.display = "none";
    document.getElementById("answerStatusContainer").style.display = "none";
    document.getElementById("roundResult").textContent = "";

    if (this.gameState.isAdmin) {
      document.getElementById("skipButtonContainer").style.display = "none";
      if (this.gameState.youtubePlayer) {
        this.gameState.youtubePlayer.stopVideo();
      }
    }
  }

  /**
   * @param {Array<[string, number]>} scoreboard
   */
  handleScorePhase(scoreboard) {
    const formattedScoreboard = scoreboard.map(([name, score]) => ({
      name,
      score
    }));
    this.gameState.totalPlayers = formattedScoreboard.length;
    this.updateLeaderboard(formattedScoreboard);

    document.getElementById("colorButtons").style.display = "none";
    document.getElementById("leaderboard").style.display = "block";
    document.getElementById("roundResult").textContent = "";
    document.getElementById("answerStatusContainer").style.display = "none";

    if (this.gameState.isAdmin) {
      document.getElementById("currentSong").innerHTML = "";
      document.getElementById("skipButtonContainer").style.display = "block";
      if (this.gameState.youtubePlayer && this.gameState.nextYoutubeId) {
        this.gameState.youtubePlayer.cueVideoById(this.gameState.nextYoutubeId);
      }
    }
  }

  /**
   * @param {Color[]} colors
   */
  async handleQuestionPhase(colors) {
    this.gameState.answeredPlayers = [];
    this.updateAnswerStatus();

    if (!this.gameState.isAdmin) {
      this.gameState.hasAnswered = false;
      this.gameState.colors = colors;
      document.getElementById("roundResult").textContent = "";
      document.getElementById("leaderboard").style.display = "none";
      this.createColorButtons(this.gameState.colors);
      document.getElementById("colorButtons").style.display = "grid";
    } else {
      document.getElementById("skipButtonContainer").style.display = "none";
      if (this.gameState.youtubePlayer) {
        // Verify and update video if needed before playing
        const videoVerified = await this.ytManager.verifyAndUpdateVideo(this.gameState.nextYoutubeId);
        if (videoVerified) {
          this.gameState.youtubePlayer.playVideo();
        } else {
          console.error('Failed to verify/update YouTube video');
          // Optionally handle the error case, maybe skip to next song
          this.gameController.skipCurrentSong();
        }
      }
    }

    document.getElementById("answerStatusContainer").style.display = "flex";
    this.startTimer(CONFIG.ROUND_DURATION * 1000);
  }

  handleAdminGameOver() {
    if (this.gameState.youtubePlayer) {
      this.gameState.youtubePlayer.stopVideo();
    }

    const adminButtons = document.querySelectorAll('#adminControls button');
    adminButtons.forEach(button => {
      if (!button.classList.contains('leave-button')) {
        button.style.display = 'none';
      }
    });

    const skipButtonContainer = document.getElementById("skipButtonContainer");
    if (skipButtonContainer) {
      skipButtonContainer.style.display = "none";
    }

    document.getElementById("currentSong").innerHTML = "";
    document.getElementById("youtubeEmbed").style.display = "none";
  }

  /**
   * @param {string} phase
   */
  updateGameControls(phase) {
    if (!this.gameState.isAdmin) return;

    const gameButton = document.getElementById('toggleGameBtn');
    const roundButton = document.getElementById('toggleRoundBtn');

    if (gameButton) {
      gameButton.textContent = this.gameState.gameStarted ? "End Game" : "Start Game";
    }

    if (roundButton) {
      if (phase === "score") {
        roundButton.textContent = this.gameState.isLoadingNextSong ? "Loading..." : "Start Round";
        roundButton.disabled = this.gameState.isLoadingNextSong;
      } else {
        roundButton.textContent = "End Round";
        roundButton.disabled = false;
      }
    }
  }

  /**
   * @param {string} message
   * @param {() => void} onConfirm
   */
  showConfirmationModal(message, onConfirm) {
    const template = document.getElementById("confirmationModalTemplate");
    const modal = template.content.cloneNode(true);

    modal.querySelector("p").textContent = message;

    // Replace onclick with addEventListener
    const confirmButton = modal.querySelector(".confirm");
    const cancelButton = modal.querySelector(".cancel");

    confirmButton.addEventListener('click', () => {
      onConfirm();
      this.closeModal();
    });

    cancelButton.addEventListener('click', () => this.closeModal());

    document.getElementById("modalContainer").appendChild(modal);
  }

  closeModal() {
    const modal = document.querySelector(".modal-overlay");
    if (modal) {
      modal.remove();
    }
  }
}

// gameController.js
class GameController {
  /**
   * @param {GameState} gameState
   * @param {WebSocketManager} wsManager
   * @param {UIManager} uiManager
   * @param {YoutubeManager} ytManager
   */
  constructor(gameState, wsManager, uiManager, ytManager) {
    this.gameState = gameState;
    this.wsManager = wsManager;
    this.uiManager = uiManager;
    this.ytManager = ytManager;

    // Remove the old bindings as we'll use event listeners instead
    this.setupEventListeners();
  }

  /**
     * @param {WebSocketManager} wsManager
     * @param {UIManager} uiManager
     */
  setManagers(wsManager, uiManager) {
    this.wsManager = wsManager;
    this.uiManager = uiManager;
  }

  setupEventListeners() {
    const elements = {
      createLobby: document.getElementById('createLobbyBtn'),
      joinLobby: document.getElementById('joinLobbyBtn'),
      leaveLobby: document.getElementById('leaveLobbyBtn'),
      toggleRound: document.getElementById('toggleRoundBtn'),
      toggleGame: document.getElementById('toggleGameBtn'),
      skipSong: document.getElementById('skipCurrentSongBtn')
    };

    // Log warning if any required elements are missing
    Object.entries(elements).forEach(([name, element]) => {
      if (!element) {
        console.warn(`Required element ${name} not found`);
      }
    });

    elements.createLobby?.addEventListener('click', () => this.createLobby());
    elements.joinLobby?.addEventListener('click', () => this.joinLobby());
    elements.leaveLobby?.addEventListener('click', () => this.leaveLobby());
    elements.toggleRound?.addEventListener('click', () => this.toggleRound());
    elements.toggleGame?.addEventListener('click', () => this.toggleGame());
    elements.skipSong?.addEventListener('click', () => this.skipCurrentSong());
  }

  /**
   * Initialize the game application
   */
  async initialize() {
    try {
      const response = await fetch("/api/lobbies");
      if (!response.ok) throw new Error("Failed to fetch lobbies");

      const data = await response.json();
      this.uiManager.updateLobbyList(data.lobbies);
    } catch (error) {
      console.error("Error fetching lobbies:", error);
      document.getElementById("lobbyList").innerHTML =
        '<p class="error-message">Failed to load lobbies.</p>';
    }
  }

  async createLobby() {
    try {
      const lobbyNameInput = document.getElementById('newLobbyName');
      const lobbyName = lobbyNameInput?.value.trim() || '';

      const response = await fetch("/api/lobbies", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          round_duration: CONFIG.ROUND_DURATION,
          lobby_name: lobbyName
        }),
      });

      if (!response.ok) {
        throw new Error("Failed to create lobby");
      }

      const data = await response.json();
      this.gameState.currentLobbyId = data.lobby_id;
      this.gameState.currentAdminId = data.admin_id;
      this.gameState.isAdmin = true;

      this.showAdminView(data.lobby_id);
      this.wsManager.connect(data.lobby_id, "Admin");

      if (typeof YT !== 'undefined' && YT.Player) {
        this.ytManager.initializePlayer('youtubeEmbed');
      }
    } catch (error) {
      console.error("Error creating lobby:", error);
      this.uiManager.showNotification("Failed to create lobby", true);
    }
  }

  /**
     * @param {string} lobbyId
     */
  showJoinForm(lobbyId) {
    this.gameState.currentLobbyId = lobbyId;
    document.getElementById("lobbySelection").style.display = "none";
    document.getElementById("selectedLobbyName").textContent = lobbyId;
    document.getElementById("joinForm").style.display = "block";
  }

  /**
   * @param {string} lobbyId 
   */
  showAdminView(lobbyId) {
    document.getElementById("lobbySelection").style.display = "none";
    document.getElementById("joinForm").style.display = "none";
    document.getElementById("lobbyInfo").style.display = "block";
    document.getElementById("adminControls").style.display = "block";
    document.getElementById("currentLobbyName").textContent = lobbyId;
    this.uiManager.updateGameControls(this.gameState.gameStarted ? "question" : "lobby");
  }

  joinLobby() {
    const playerNameInput = document.getElementById("playerName").value.trim();
    if (!playerNameInput) {
      this.uiManager.showNotification("Please enter your name", true);
      return;
    }

    if (!this.gameState.currentLobbyId) {
      this.uiManager.showNotification("No lobby selected", true);
      return;
    }

    this.gameState.playerName = playerNameInput;
    this.wsManager.connect(this.gameState.currentLobbyId, playerNameInput);
  }

  leaveLobby() {
    if (this.gameState.isAdmin) {
      this.uiManager.showConfirmationModal(
        "Are you sure you want to leave? This will close the lobby for all players.",
        () => {
          this.wsManager.sendMessage({
            type: "AdminAction",
            lobby_id: this.gameState.currentLobbyId,
            action: {
              type: "CloseGame",
              reason: "Admin closed the lobby"
            }
          });
          this.handleGameClosed("Admin closed the lobby");
        }
      );
    } else {
      this.wsManager.sendMessage({
        type: "Leave",
        lobby_id: this.gameState.currentLobbyId,
      });
      this.wsManager.closeConnection();
      this.uiManager.resetGameState();
      this.gameState.currentLobbyId = null;
      this.ytManager.cleanup();
      document.getElementById("lobbyInfo").style.display = "none";
      document.getElementById("lobbySelection").style.display = "block";
    }
  }

  /**
   * @param {string} reason
   */
  handleGameClosed(reason) {
    this.uiManager.showNotification(`Game Closed: ${reason}`, true);
    this.wsManager.closeConnection();
    this.uiManager.resetGameState();
    this.gameState.isAdmin = false;
    this.gameState.gameStarted = false;
    document.getElementById("lobbyInfo").style.display = "none";
    document.getElementById("adminControls").style.display = "none";
    document.getElementById("lobbySelection").style.display = "block";
    this.initialize();
  }

  toggleGame() {
    if (!this.gameState.socket || !this.gameState.isAdmin) return;

    const action = {
      type: "AdminAction",
      lobby_id: this.gameState.currentLobbyId,
      action: {
        type: this.gameState.gameStarted ? "EndGame" : "StartGame"
      }
    };

    if (this.gameState.gameStarted) {
      action.action.reason = "Game ended by admin";
    }

    this.wsManager.sendMessage(action);
  }

  toggleRound() {
    if (!this.gameState.socket || !this.gameState.isAdmin) return;

    const currentPhase = document.getElementById("gameState").textContent;

    if (currentPhase.includes("score")) {
      if (!this.gameState.nextYoutubeId) {
        this.uiManager.showNotification("Waiting for next song to load...", false);
        return;
      }

      if (this.gameState.isLoadingNextSong) {
        this.uiManager.showNotification("Please wait for next song to load...", false);
        return;
      }

      this.startRound();
    } else if (currentPhase.includes("question")) {
      this.endRound();
    }
  }

  startRound() {
    if (this.gameState.socket && this.gameState.isAdmin) {
      this.wsManager.sendMessage({
        type: "AdminAction",
        lobby_id: this.gameState.currentLobbyId,
        action: {
          type: "StartRound",
          colors: null
        }
      });
    }
  }

  endRound() {
    if (this.gameState.socket && this.gameState.isAdmin) {
      this.wsManager.sendMessage({
        type: "AdminAction",
        lobby_id: this.gameState.currentLobbyId,
        action: {
          type: "EndRound"
        }
      });
    }
  }

  skipCurrentSong() {
    if (this.gameState.socket && this.gameState.isAdmin) {
      this.wsManager.sendMessage({
        type: "AdminAction",
        lobby_id: this.gameState.currentLobbyId,
        action: {
          type: "SkipSong"
        }
      });
    }
  }

  /**
   * @param {string} colorName
   */
  selectColor(colorName) {
    if (!this.gameState.hasAnswered) {
      this.wsManager.sendMessage({
        type: "Answer",
        lobby_id: this.gameState.currentLobbyId,
        color: colorName,
      });
      this.gameState.hasAnswered = true;
      this.gameState.playerAnswer = colorName;
      this.uiManager.stopTimer();
      this.uiManager.createColorButtons(this.gameState.colors);
    }
  }
}

// Initialize the application
document.addEventListener("DOMContentLoaded", () => {
  const gameState = new GameState();
  const ytManager = new YoutubeManager(gameState);

  const gameController = new GameController(gameState, null, null, ytManager);
  const uiManager = new UIManager(gameState, ytManager, gameController);
  const wsManager = new WebSocketManager(gameState, uiManager);

  gameController.setManagers(wsManager, uiManager);
  gameController.initialize();

  // Create YouTube player container for admin
  if (gameState.isAdmin) {
    const youtubeContainer = document.getElementById("youtubeEmbed");
    if (!youtubeContainer) {
      const container = document.createElement("div");
      container.id = "youtubeEmbed";
      document.getElementById("adminControls").appendChild(container);
    }
  }
});

// YouTube API callback
window.onYouTubeIframeAPIReady = function() {
  if (window.game?.gameState.isAdmin) {
    window.game.ytManager.initializePlayer('youtubeEmbed');
  }
};
