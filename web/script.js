let socket = null;
let playerName = "";
let currentLobbyId = null;
let currentAdminId = null;
let isAdmin = false;
let colors = [];
let playerAnswer = null;
let roundDuration = 60;
let hasAnswered = false;
let playerScore = 0;
let totalPlayers = 0;
let answeredPlayers = [];

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
      // Admin only needs to connect to control the game
      const joinMsg = {
        type: "JoinLobby",
        lobby_id: lobbyId,
        admin_id: currentAdminId,  // This identifies them as admin
        name: "Admin"  // We don't really need a name for admin
      };
      socket.send(JSON.stringify(joinMsg));
    } else {
      // Regular player join
      const joinMsg = {
        type: "JoinLobby",
        lobby_id: lobbyId,
        admin_id: null,  // Explicitly set to null for players
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
      if (isAdmin) {
        console.log(data.current_song_name, data.current_song_artist);
      }
      handleStateChanged(data.phase, data.colors, data.scoreboard);
      break;
    case "GameOver":
      handleGameOver(data.scores, data.reason);
      break;
    case "GameClosed":
      handleGameClosed(data.reason);
      break;
    case "Error":
      showNotification(data.message, true);
      break;
    default:
      console.warn("Unhandled server message type:", data.type);
  }
}

function updateInitialPlayerList(players) {
  totalPlayers = players.length;
  // The format of players has changed - now it's an object with name and players array
  if (players.players) {
    updateLeaderboard(players.players);
  } else {
    // Handle directly passed array format
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

  if (phase === "lobby") {
    document.getElementById("colorButtons").style.display = "none";
    document.getElementById("leaderboard").style.display = "none";
    document.getElementById("answerStatusContainer").style.display = "none";
    document.getElementById("roundResult").textContent = "";
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
    }
    document.getElementById("answerStatusContainer").style.display = "flex";
    startTimer(roundDuration * 1000);
  } else if (phase === "score") {
    const formattedScoreboard = scoreboard.map(([name, score]) => ({
      name,
      score
    }));
    updateLeaderboard(formattedScoreboard);
    document.getElementById("colorButtons").style.display = "none";
    document.getElementById("leaderboard").style.display = "block";
    document.getElementById("roundResult").textContent = "";
    document.getElementById("answerStatusContainer").style.display = "none";
  }
}

function handleGameOver(finalScores, reason) {
  stopTimer();
  showNotification(`Game Over: ${reason}`, true);
  updateLeaderboard(finalScores.map(([name, score]) => ({ name, score })));
}

function handleGameClosed(reason) {
  showNotification(`Game Closed: ${reason}`, true);
  closeConnection();
  resetUIState();
  isAdmin = false;
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

function showAdminView(lobbyId) {
  document.getElementById("lobbySelection").style.display = "none";
  document.getElementById("joinForm").style.display = "none";
  document.getElementById("lobbyInfo").style.display = "block";
  document.getElementById("adminControls").style.display = "block";
  document.getElementById("currentLobbyName").textContent = lobbyId;
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
    // If we're in score phase, start a round. If we're in question phase, end the round
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
    button.style.backgroundColor = color.rgb;
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

  // Store the interval ID so we can stop it later if needed
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

document.addEventListener("DOMContentLoaded", initializeApp);
