// Socket and game state variables
let socket = null;
let playerName = "";
let currentLobbyId = null;
let currentAdminId = null;
let gameStarted = false;
let isAdmin = false;
let colors = [];
let playerAnswer = null;
let roundDuration = 60;
let hasAnswered = false;
let playerScore = 0;
let totalPlayers = 0;
let answeredPlayers = [];
let youtubePlayer = null;
let nextYoutubeId = null;

// Load YouTube IFrame Player API
const tag = document.createElement('script');
tag.src = "https://www.youtube.com/iframe_api";
const firstScriptTag = document.getElementsByTagName('script')[0];
firstScriptTag.parentNode.insertBefore(tag, firstScriptTag);

// Called automatically by YouTube API when ready
function onYouTubeIframeAPIReady() {
  if (isAdmin) {
    initializeYoutubePlayer();
  }
}

function initializeYoutubePlayer() {
  youtubePlayer = new YT.Player('youtubeEmbed', {
    height: '100%',
    width: '100%',
    playerVars: {
      'controls': 0,
      'playsinline': 1,
      'enablejsapi': 1
    },
    events: {
      'onReady': onPlayerReady,
      'onStateChange': onPlayerStateChange
    }
  });
}

function onPlayerReady(event) {
  if (nextYoutubeId) {
    event.target.cueVideoById(nextYoutubeId);
  }
}

function onPlayerStateChange(event) {
  // we aren't using this right now, but it could be useful in the future
}

async function createLobby() {
  try {
    const response = await fetch("/api/lobbies", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ round_duration: roundDuration }),
    });

    if (!response.ok) {
      throw new Error("Failed to create lobby");
    }

    const data = await response.json();
    currentLobbyId = data.lobby_id;
    currentAdminId = data.admin_id;
    isAdmin = true;
    showAdminView(currentLobbyId);
    connectToLobby(currentLobbyId, playerName, isAdmin);

    if (typeof YT !== 'undefined' && YT.Player) {
      initializeYoutubePlayer();
    }
  } catch (error) {
    console.error("Error creating lobby:", error);
    showNotification("Failed to create lobby", true);
  }
}

async function initializeApp() {
  try {
    const response = await fetch("/api/lobbies");
    if (!response.ok) {
      throw new Error("Failed to fetch lobbies");
    }
    const data = await response.json();
    updateLobbyList(data.lobbies);
  } catch (error) {
    console.error("Error fetching lobbies:", error);
    document.getElementById("lobbyList").innerHTML =
      '<p class="error-message">Failed to load lobbies.</p>';
  }
}

function updateLobbyList(lobbyIds) {
  const lobbyList = document.getElementById("lobbyList");
  lobbyList.innerHTML = "";

  if (lobbyIds.length === 0) {
    lobbyList.innerHTML =
      "<p>No active lobbies. Create one to get started!</p>";
    return;
  }

  lobbyIds.forEach((id) => {
    const lobbyElement = document.createElement("div");
    lobbyElement.className = "lobby-item";
    lobbyElement.innerHTML = `
      <div class="lobby-info">
        <span>Lobby: ${id}</span>
      </div>
      <button onclick="showJoinForm('${id}')">Join</button>
    `;
    lobbyList.appendChild(lobbyElement);
  });
}

function showJoinForm(lobbyId) {
  currentLobbyId = lobbyId;
  document.getElementById("lobbySelection").style.display = "none";
  document.getElementById("selectedLobbyName").textContent = lobbyId;
  document.getElementById("joinForm").style.display = "block";
}

function joinLobby() {
  playerName = document.getElementById("playerName").value.trim();
  if (!playerName) {
    showNotification("Please enter your name", true);
    return;
  }

  if (!currentLobbyId) {
    showNotification("No lobby selected", true);
    return;
  }

  connectToLobby(currentLobbyId, playerName, false);
}

function connectToLobby(lobbyId, name) {
  closeConnection();
  const wsProtocol = window.location.protocol === "https:" ? "wss:" : "ws:";
  const wsUrl = `${wsProtocol}//${window.location.host}/ws`;
  socket = new WebSocket(wsUrl);

  socket.onopen = () => {
    if (isAdmin) {
      const joinMsg = {
        type: "JoinLobby",
        lobby_id: lobbyId,
        admin_id: currentAdminId,
        name: "Admin"
      };
      socket.send(JSON.stringify(joinMsg));
    } else {
      const joinMsg = {
        type: "JoinLobby",
        lobby_id: lobbyId,
        admin_id: null,
        name: name,
      };
      socket.send(JSON.stringify(joinMsg));
    }

    document.getElementById("joinForm").style.display = "none";
    document.getElementById("lobbyInfo").style.display = "block";
    resetUIState();
  };

  socket.onmessage = handleServerMessage;

  socket.onclose = () => {
    showNotification("Disconnected from lobby.", true);
  };

  socket.onerror = (error) => {
    console.error("WebSocket error:", error);
    showNotification("Connection error", true);
  };
}

function handleServerMessage(event) {
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
      roundDuration = data.round_duration;
      updateLeaderboard(data.players);
      break;
    case "InitialPlayerList":
      updateLeaderboard(data.players.map(([name, score]) => ({ name, score })));
      break;
    case "PlayerJoined":
      playerJoined(data.player_name, data.current_score);
      break;
    case "PlayerLeft":
      playerLeft(data.name);
      break;
    case "PlayerAnswered":
      handlePlayerAnswered(data.name, data.correct, data.new_score);
      break;
    case "StateChanged":
      handleStateChanged(data.phase, data.colors, data.scoreboard);
      break;
    case "GameOver":
      handleGameOver(data.scores, data.reason);
      break;
    case "GameClosed":
      handleGameClosed(data.reason);
      break;
    case "AdminInfo":
      handleAdminInfo(data.current_song_name, data.current_song_artist, data.current_song_youtube_id);
      break;
    case "AdminNextSongs":
      handleAdminNextSongs(data.upcoming_songs);
      break;
    case "Error":
      showNotification(data.message, true);
      break;
    default:
      console.warn("Unhandled server message type:", data.type);
  }
}

function skipCurrentSong() {
  if (socket && isAdmin) {
    sendMessage({
      type: "AdminAction",
      lobby_id: currentLobbyId,
      action: {
        type: "SkipSong"
      }
    });
  }
}

function handleAdminNextSongs(upcomingSongs) {
  if (isAdmin && upcomingSongs && upcomingSongs.length > 0) {
    const nextSong = upcomingSongs[0];
    nextYoutubeId = nextSong.youtube_id;

    // If we're in score phase and the player is ready, preload the video
    if (youtubePlayer && youtubePlayer.getPlayerState) {
      const currentPhase = document.getElementById("gameState").textContent;
      if (currentPhase.includes("score")) {
        youtubePlayer.cueVideoById(nextYoutubeId);
      }
    }

    // Update the upcoming songs list
    const songsList = document.getElementById("songsList");
    songsList.innerHTML = "";

    upcomingSongs.forEach((song, index) => {
      const songElement = document.createElement("div");
      songElement.className = "upcoming-song";
      songElement.innerHTML = `
        <span class="song-number">${index + 1}.</span>
        <span class="song-info">${song.song_name} - ${song.artist}</span>
      `;
      songsList.appendChild(songElement);
    });
  }
}

function updateInitialPlayerList(players) {
  totalPlayers = players.length;
  if (players.players) {
    updateLeaderboard(players.players);
  } else {
    updateLeaderboard(players.map(([name, score]) => ({ name, score })));
  }
}

function playerJoined(name, score) {
  totalPlayers += 1;
  showNotification(`${name} joined the game`);
}

function playerLeft(name) {
  totalPlayers = Math.max(0, totalPlayers - 1);
  showNotification(`${name} left the game`);
}

function handlePlayerAnswered(name, correct, newScore) {
  if (name === playerName) {
    playerScore = newScore;
    updateYourScore(playerScore);
  }
  answeredPlayers.push({ playerName: name, correct });
  updateAnswerStatus(answeredPlayers.length, totalPlayers);
}

function updateAnswerStatus(answeredCount, totalPlayers) {
  const counterElement = document.getElementById("answerCounter");
  counterElement.textContent = `${answeredCount}/${totalPlayers}`;

  const answeredPlayersElement = document.getElementById("answeredPlayers");
  answeredPlayersElement.innerHTML = "";

  answeredPlayers.forEach((p) => {
    const playerSpan = document.createElement("span");
    playerSpan.className = "answered-player";
    playerSpan.style.backgroundColor = p.correct ? "#35cf0e" : "#cf0e22";
    playerSpan.textContent = p.playerName;
    answeredPlayersElement.appendChild(playerSpan);
  });
}

function handleStateChanged(phase, newColors, scoreboard) {
  document.getElementById("gameState").textContent = `Current Phase: ${phase}`;
  stopTimer();

  // Set game started state based on phase
  if (phase !== "lobby") {
    gameStarted = true;
  } else {
    gameStarted = false;
  }

  // Update button texts if we're admin
  if (isAdmin) {
    updateButtonTexts();
  }

  const skipButtonContainer = document.getElementById("skipButtonContainer");

  if (phase === "lobby") {
    document.getElementById("colorButtons").style.display = "none";
    document.getElementById("leaderboard").style.display = "none";
    document.getElementById("answerStatusContainer").style.display = "none";
    document.getElementById("roundResult").textContent = "";
    if (isAdmin) {
      skipButtonContainer.style.display = "none";
      if (youtubePlayer) {
        youtubePlayer.stopVideo();
      }
    }
  } else if (phase === "score") {
    const formattedScoreboard = scoreboard.map(([name, score]) => ({
      name,
      score
    }));
    totalPlayers = formattedScoreboard.length;
    updateLeaderboard(formattedScoreboard);
    document.getElementById("colorButtons").style.display = "none";
    document.getElementById("leaderboard").style.display = "block";
    document.getElementById("roundResult").textContent = "";
    document.getElementById("answerStatusContainer").style.display = "none";

    if (isAdmin) {
      document.getElementById("currentSong").innerHTML = "";
      skipButtonContainer.style.display = "block";
      if (youtubePlayer && nextYoutubeId) {
        youtubePlayer.cueVideoById(nextYoutubeId);
      }
    }
  } else if (phase === "question") {
    answeredPlayers = [];
    updateAnswerStatus(0, totalPlayers);
    if (!isAdmin) {
      hasAnswered = false;
      colors = newColors;
      document.getElementById("roundResult").textContent = "";
      document.getElementById("leaderboard").style.display = "none";
      createColorButtons(colors);
      document.getElementById("colorButtons").style.display = "grid";
    } else {
      skipButtonContainer.style.display = "none";
      if (youtubePlayer) {
        youtubePlayer.playVideo();
      }
    }
    document.getElementById("answerStatusContainer").style.display = "flex";
    startTimer(roundDuration * 1000);
  }
}

function updateGameButton() {
  const gameButton = document.querySelector('button[onclick="toggleGame()"]');
  if (gameButton) {
    gameButton.textContent = gameStarted ? "End Game" : "Start Game";
  }
}

function updateButtonTexts() {
  if (!isAdmin) return;

  const gameButton = document.querySelector('button[onclick="toggleGame()"]');
  const roundButton = document.querySelector('button[onclick="toggleRound()"]');

  if (gameButton) {
    gameButton.textContent = gameStarted ? "End Game" : "Start Game";
  }

  if (roundButton) {
    const phase = document.getElementById("gameState").textContent;
    roundButton.textContent = phase.includes("score") ? "Start Round" : "End Round";
  }
}

function toggleGame() {
  if (socket && isAdmin) {
    if (!gameStarted) {
      sendMessage({
        type: "AdminAction",
        lobby_id: currentLobbyId,
        action: {
          type: "StartGame"
        }
      });
    } else {
      sendMessage({
        type: "AdminAction",
        lobby_id: currentLobbyId,
        action: {
          type: "EndGame",
          reason: "Game ended by admin"
        }
      });
    }
  }
}

function handleGameOver(scores, reason) {
  stopTimer();
  showNotification(`Game Over: ${reason}`, true);

  document.getElementById("colorButtons").style.display = "none";
  document.getElementById("answerStatusContainer").style.display = "none";
  document.getElementById("roundResult").textContent = "";

  document.getElementById("leaderboard").style.display = "block";

  const formattedScores = scores.map(([name, score]) => ({
    name: name,
    score: score
  }));
  updateLeaderboard(formattedScores);

  if (isAdmin) {
    if (youtubePlayer) {
      youtubePlayer.stopVideo();
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

  // Update game state
  gameStarted = false;
  document.getElementById("gameState").textContent = "Game Over";
}

function handleGameClosed(reason) {
  showNotification(`Game Closed: ${reason}`, true);
  closeConnection();
  resetUIState();
  isAdmin = false;
  gameStarted = false;
  document.getElementById("lobbyInfo").style.display = "none";
  document.getElementById("adminControls").style.display = "none";
  document.getElementById("lobbySelection").style.display = "block";
  initializeApp();
}


function leaveLobby() {
  if (isAdmin) {
    showConfirmationModal(
      "Are you sure you want to leave? This will close the lobby for all players.",
      () => {
        sendMessage({
          type: "AdminAction",
          lobby_id: currentLobbyId,
          action: {
            type: "CloseGame",
            reason: "Admin closed the lobby"
          }
        });
        handleGameClosed("Admin closed the lobby");
      },
    );
  } else {
    sendMessage({
      type: "Leave",
      lobby_id: currentLobbyId,
    });
    closeConnection();
    resetUIState();
    currentLobbyId = null;
    document.getElementById("lobbyInfo").style.display = "none";
    document.getElementById("lobbySelection").style.display = "block";
  }
}

function closeConnection() {
  if (socket) {
    socket.close();
    socket = null;
  }
}

// Update showAdminView to set initial button text
function showAdminView(lobbyId) {
  document.getElementById("lobbySelection").style.display = "none";
  document.getElementById("joinForm").style.display = "none";
  document.getElementById("lobbyInfo").style.display = "block";
  document.getElementById("adminControls").style.display = "block";
  document.getElementById("currentLobbyName").textContent = lobbyId;
  updateButtonTexts();
}

function sendMessage(message) {
  if (socket && socket.readyState === WebSocket.OPEN) {
    console.log("sending:", message);
    socket.send(JSON.stringify(message));
  } else {
    showNotification("Connection not available", true);
  }
}

function startRound() {
  if (socket && isAdmin) {
    sendMessage({
      type: "AdminAction",
      lobby_id: currentLobbyId,
      action: {
        type: "StartRound",
        colors: null
      }
    });
  }
}

function endRound() {
  if (socket && isAdmin) {
    sendMessage({
      type: "AdminAction",
      lobby_id: currentLobbyId,
      action: {
        type: "EndRound"
      }
    });
  }
}

function toggleRound() {
  if (socket && isAdmin) {
    const currentPhase = document.getElementById("gameState").textContent;
    if (currentPhase.includes("score")) {
      startRound();
    } else if (currentPhase.includes("question")) {
      endRound();
    }
  }
}

function startGame() {
  if (socket && isAdmin) {
    sendMessage({
      type: "AdminAction",
      lobby_id: currentLobbyId,
      action: {
        type: "EndRound"
      }
    });
  }
}

function toggleRound() {
  if (socket && isAdmin) {
    const currentPhase = document.getElementById("gameState").textContent;
    if (currentPhase.includes("score")) {
      startRound();
    } else if (currentPhase.includes("question")) {
      endRound();
    }
  }
}

function startGame() {
  if (socket && isAdmin) {
    sendMessage({
      type: "AdminAction",
      lobby_id: currentLobbyId,
      action: {
        type: "StartGame"
      }
    });
  }
}

function endGame(reason = "Game ended by admin") {
  if (socket && isAdmin) {
    sendMessage({
      type: "AdminAction",
      lobby_id: currentLobbyId,
      action: {
        type: "EndGame",
        reason
      }
    });
  }
}

function selectColor(colorName) {
  if (!hasAnswered) {
    sendMessage({
      type: "Answer",
      lobby_id: currentLobbyId,
      color: colorName,
    });
    hasAnswered = true;
    playerAnswer = colorName;
    stopTimer();
    createColorButtons(colors);
  }
}

function resetUIState() {
  playerScore = 0;
  document.getElementById("yourScore").textContent = "0";
  document.getElementById("leaderboard").innerHTML = "";
  document.getElementById("roundResult").textContent = "";
  document.getElementById("colorButtons").innerHTML = "";
  document.getElementById("answerStatusContainer").style.display = "none";
}

function createColorButtons(colors) {
  const colorButtons = document.getElementById("colorButtons");
  colorButtons.innerHTML = "";
  colors.forEach((color) => {
    const button = document.createElement("button");
    button.className = "color-button";

    // Apply metallic effects for gold and silver
    if (color.name.toLowerCase() === "gold") {
      button.classList.add("metallic-gold");
    } else if (color.name.toLowerCase() === "silver") {
      button.classList.add("metallic-silver");
    } else {
      button.style.backgroundColor = color.rgb;
    }

    button.onclick = () => selectColor(color.name);
    button.title = color.name;

    if (hasAnswered && playerAnswer === color.name) {
      button.style.border = "3px solid white";
    }
    if (hasAnswered) {
      button.disabled = true;
    }

    colorButtons.appendChild(button);
  });
}

function updateYourScore(score) {
  document.getElementById("yourScore").textContent = score;
}

function updateLeaderboard(players) {
  const leaderboard = document.getElementById("leaderboard");
  leaderboard.innerHTML = "<h2>Leaderboard</h2>";
  players.sort((a, b) => b.score - a.score);
  const maxScore = players.length > 0 ? players[0].score : 0;

  players.forEach((player, index) => {
    const playerItem = document.createElement("div");
    playerItem.className = "player-item";
    const playerInfo = document.createElement("div");
    playerInfo.className = "player-info";
    playerInfo.innerHTML = `<span>${index + 1}. ${player.name}</span><span>${player.score} points</span>`;

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

function handleAdminInfo(song, artist, youtube_id) {
  const song_el = document.getElementById("currentSong");
  song_el.textContent = song + " by " + artist;

  nextYoutubeId = youtube_id;

  // If we're in score phase and the player is ready, preload the video
  if (isAdmin && youtubePlayer && youtubePlayer.getPlayerState) {
    const currentPhase = document.getElementById("gameState").textContent;
    if (currentPhase.includes("score")) {
      youtubePlayer.cueVideoById(youtube_id);
    }
  }
}

function showNotification(message, isError = false) {
  const notification = document.createElement("div");
  notification.className = `notification ${isError ? "error" : "info"}`;
  notification.textContent = message;
  document.body.appendChild(notification);

  setTimeout(() => {
    notification.remove();
  }, 3000);
}

function showConfirmationModal(message, onConfirm) {
  const template = document.getElementById("confirmationModalTemplate");
  const modal = template.content.cloneNode(true);

  modal.querySelector("p").textContent = message;
  modal.querySelector(".confirm").onclick = () => {
    onConfirm();
    closeModal();
  };
  modal.querySelector(".cancel").onclick = closeModal;

  document.getElementById("modalContainer").appendChild(modal);
}

function closeModal() {
  const modal = document.querySelector(".modal-overlay");
  if (modal) {
    modal.remove();
  }
}

function startTimer(durationMs) {
  let timeLeftMs = durationMs;
  const timerElem = document.getElementById("timer");
  const interval = setInterval(() => {
    timeLeftMs -= 100;
    if (timeLeftMs <= 0) {
      clearInterval(interval);
      timerElem.textContent = "Time's up!";
    } else {
      const seconds = (timeLeftMs / 1000).toFixed(1);
      timerElem.textContent = `Time remaining: ${seconds}s`;
    }
  }, 100);

  timerElem.dataset.intervalId = interval;
}

function stopTimer() {
  const timerElem = document.getElementById("timer");
  const interval = timerElem.dataset.intervalId;
  if (interval) {
    clearInterval(interval);
    delete timerElem.dataset.intervalId;
  }
  timerElem.textContent = "Answer Received!";
}

document.addEventListener("DOMContentLoaded", () => {
  initializeApp();

  // Create YouTube player container for admin
  if (isAdmin) {
    const youtubeContainer = document.getElementById("youtubeEmbed");
    if (!youtubeContainer) {
      const container = document.createElement("div");
      container.id = "youtubeEmbed";
      document.getElementById("adminControls").appendChild(container);
    }
  }
});
