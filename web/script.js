let socket = null;
let playerName = "";
let currentLobbyId = null;
let roundStartTime;
let roundDuration;
let timerInterval;
let hasAnswered = false;
let playerAnswer = null;
let colors = [];
let answeredPlayers = [];
let totalPlayers = 0;
let isAdmin = false;
let lobbyClosing = false;

async function createLobby() {
  const lobbyName = document.getElementById("newLobbyName").value;
  try {
    const response = await fetch("/api/lobbies", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        name: lobbyName || null,
      }),
    });

    if (!response.ok) {
      throw new Error("Failed to create lobby");
    }

    const lobby = await response.json();
    isAdmin = true;
    showAdminView(lobby);
  } catch (error) {
    console.error("Error creating lobby:", error);
    showNotification("Failed to create lobby. Please try again.", true);
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

function showConfirmationModal(message, onConfirm, onCancel) {
  const template = document.getElementById("confirmationModalTemplate");
  const modal = template.content.cloneNode(true);

  modal.querySelector("p").textContent = message;
  modal.querySelector(".confirm").onclick = () => {
    onConfirm();
    closeModal();
  };
  modal.querySelector(".cancel").onclick = onCancel || closeModal;

  document.getElementById("modalContainer").appendChild(modal);
}

function closeModal() {
  const modal = document.querySelector(".modal-overlay");
  if (modal) {
    modal.remove();
  }
}

function showJoinForm(lobbyId, lobbyName) {
  currentLobbyId = lobbyId;
  document.getElementById("lobbySelection").style.display = "none";
  document.getElementById("selectedLobbyName").textContent = lobbyName;
  document.getElementById("joinForm").style.display = "block";
}

function connectToLobby(lobbyId) {
  if (socket) {
    socket.close();
  }

  const wsProtocol = window.location.protocol === "https:" ? "wss:" : "ws:";
  const wsUrl = `${wsProtocol}//${window.location.host}/ws?lobby=${lobbyId}`;
  socket = new WebSocket(wsUrl);

  socket.onopen = function () {
    socket.send(
      JSON.stringify({
        action: "join",
        name: playerName,
      }),
    );
    document.getElementById("joinForm").style.display = "none";
    document.getElementById("lobbyInfo").style.display = "block";
  };

  setupWebSocketHandlers();
}

function setupWebSocketHandlers() {
  socket.onmessage = function (event) {
    try {
      const data = JSON.parse(event.data);
      console.log("Received message:", data);

      switch (data.action) {
        case "game_state":
          handleGameState(data);
          break;
        case "color_result":
          handleColorResult(data);
          break;
        case "update_answer_count":
          updateAnswerStatus(data.answeredCount, data.totalPlayers);
          break;
        case "player_answered":
          handlePlayerAnswer(data.playerName, data.correct);
          break;
        case "state_updated":
          break;
        case "lobby_closing":
          handleLobbyClosing(data.reason);
          break;
        case "error":
          handleError(data.message);
          break;
        default:
          console.warn("Unhandled message type:", data.action);
      }
    } catch (error) {
      console.error("Error processing message:", error);
      showNotification("Error processing server message", true);
    }
  };

  socket.onclose = function (event) {
    if (lobbyClosing) {
      lobbyClosing = false;
      return;
    }

    console.log("WebSocket connection closed. Attempting to reconnect...");
    showNotification("Connection lost. Attempting to reconnect...", true);

    setTimeout(() => {
      if (currentLobbyId && playerName) {
        connectToLobby(currentLobbyId);
        socket.onopen = function () {
          socket.send(
            JSON.stringify({
              action: "join",
              name: playerName,
            }),
          );
          showNotification("Reconnected to lobby");
        };
      }
    }, 3000);
  };

  socket.onerror = function (error) {
    console.error("WebSocket error:", error);
    showNotification("Connection error occurred", true);
  };
}

function joinLobby() {
  playerName = document.getElementById("playerName").value;
  if (!playerName) {
    showNotification("Please enter your name", true);
    return;
  }

  if (!currentLobbyId) {
    showNotification("No lobby selected", true);
    return;
  }

  connectToLobby(currentLobbyId);
}

function leaveLobby() {
  if (isAdmin) {
    showConfirmationModal(
      "Are you sure you want to leave? This will close the lobby for all players.",
      confirmAdminLeave,
      () => closeModal(),
    );
  } else {
    if (socket) {
      socket.close();
    }
    currentLobbyId = null;
    playerName = "";
    document.getElementById("lobbyInfo").style.display = "none";
    document.getElementById("lobbySelection").style.display = "block";
    resetGameState();
    initializeApp();
  }
}

function confirmAdminLeave() {
  lobbyClosing = true;
  adminLeaveLobby();
}

function adminLeaveLobby() {
  if (socket && socket.readyState === WebSocket.OPEN) {
    sendMessage({ action: "close_lobby" });
    handleLobbyClosing("Admin closed the lobby");
  } else {
    showNotification("Unable to close lobby: Connection Lost", true);
  }
}

function handleLobbyClosing(reason) {
  showNotification(reason, true);
  lobbyClosing = true;

  if (socket) {
    socket.close();
  }

  resetGameState();
  isAdmin = false;

  document.getElementById("lobbyInfo").style.display = "none";
  document.getElementById("adminControls").style.display = "none";
  document.getElementById("lobbySelection").style.display = "block";

  currentLobbyId = null;
  playerName = "";

  initializeApp();
}

function showAdminView(lobby) {
  document.getElementById("lobbySelection").style.display = "none";
  document.getElementById("joinForm").style.display = "none";
  document.getElementById("lobbyInfo").style.display = "block";
  document.getElementById("adminControls").style.display = "block";
  document.getElementById("currentLobbyName").textContent =
    lobby.name || "Unnamed Lobby";
  connectToLobbyAsAdmin(lobby.id);
}

function connectToLobbyAsAdmin(lobbyId) {
  if (socket) {
    socket.close();
  }

  currentLobbyId = lobbyId;
  const wsProtocol = window.location.protocol === "https:" ? "wss:" : "ws:";
  const wsUrl = `${wsProtocol}//${window.location.host}/ws?lobby=${lobbyId}&role=admin`;
  socket = new WebSocket(wsUrl);

  setupWebSocketHandlers();
}

function sendMessage(message) {
  try {
    if (socket && socket.readyState === WebSocket.OPEN) {
      socket.send(JSON.stringify(message));
    } else {
      showNotification("Connection not available", true);
    }
  } catch (error) {
    console.error("Error sending message:", error);
    showNotification("Failed to send message to server", true);
  }
}

function toggleRound() {
  if (socket && isAdmin) {
    sendMessage({ action: "toggle_state" });
  }
}

function resetGameState() {
  hasAnswered = false;
  playerAnswer = null;
  colors = [];
  answeredPlayers = [];
  totalPlayers = 0;
  stopTimer();
  document.getElementById("colorButtons").innerHTML = "";
  document.getElementById("leaderboard").innerHTML = "";
  document.getElementById("roundResult").textContent = "";
  document.getElementById("yourScore").textContent = "0";
}

function handleGameState(data) {
  document.getElementById("gameState").textContent =
    `Current Phase: ${data.state.charAt(0).toUpperCase() + data.state.slice(1)}`;
  updateYourScore(data.score);

  if (data.state === "question") {
    hasAnswered = data.hasAnswered;
    playerAnswer = data.answer;
    colors = data.colors;
    answeredPlayers = [];
    totalPlayers = data.totalPlayers;

    createColorButtons(colors);
    document.getElementById("colorButtons").style.display = "grid";
    document.getElementById("leaderboard").style.display = "none";
    document.getElementById("roundResult").textContent = "";
    document.getElementById("answerStatusContainer").style.display = "flex";
    updateAnswerStatus(data.answeredCount, data.totalPlayers);

    let timeLeftMs = data.roundTimeLeft * 1000 || 0;
    startTimer(timeLeftMs);
  } else if (data.state === "score") {
    document.getElementById("colorButtons").style.display = "none";
    updateLeaderboard(data.leaderboard);
    document.getElementById("leaderboard").style.display = "block";
    document.getElementById("answerStatusContainer").style.display = "none";
    stopTimer();
  }
}

function startTimer(timeLeftMs) {
  stopTimer();
  timerInterval = setInterval(() => {
    timeLeftMs -= 10;
    if (timeLeftMs <= 0) {
      stopTimer();
      document.getElementById("timer").textContent = "Time's up!";
      document.getElementById("colorButtons").style.display = "none";
    } else {
      const secondsRemaining = (timeLeftMs / 1000).toFixed(2);
      document.getElementById("timer").textContent =
        `Time remaining: ${secondsRemaining}s`;
    }
  }, 10);
}

function stopTimer() {
  clearInterval(timerInterval);
  document.getElementById("timer").textContent = "";
}

function updateYourScore(score) {
  document.getElementById("yourScore").textContent = score;
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

function selectColor(colorName) {
  if (!hasAnswered) {
    sendMessage({ action: "select_color", color: colorName });
    hasAnswered = true;
    playerAnswer = colorName;
    stopTimer();
    createColorButtons(colors);
  }
}

function handleColorResult(data) {
  updateYourScore(data.totalScore);
  const resultText = data.correct
    ? `Correct! You earned ${data.score} points this round.`
    : `Wrong color. You earned 0 points this round.`;
  document.getElementById("roundResult").textContent = resultText;
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

function handlePlayerAnswer(playerName, correct) {
  if (!answeredPlayers.some((p) => p.playerName === playerName)) {
    answeredPlayers.push({ playerName, correct });
    updateAnswerStatus(answeredPlayers.length, totalPlayers);
  }
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

function handleError(message) {
  console.error("Error:", message);
  showNotification(message, true);
}

async function initializeApp() {
  try {
    const response = await fetch("/api/lobbies");
    if (!response.ok) {
      throw new Error("Failed to fetch lobbies");
    }
    const lobbies = await response.json();
    updateLobbyList(lobbies);
  } catch (error) {
    console.error("Error fetching lobbies:", error);
    document.getElementById("lobbyList").innerHTML =
      '<p class="error-message">Failed to load lobbies. Please try again later.</p>';
    showNotification("Failed to load lobbies", true);
  }
}

function updateLobbyList(lobbies) {
  const lobbyList = document.getElementById("lobbyList");
  lobbyList.innerHTML = "";

  if (lobbies.length === 0) {
    lobbyList.innerHTML =
      "<p>No active lobbies. Create one to get started!</p>";
    return;
  }

  lobbies.forEach((lobby) => {
    const lobbyElement = document.createElement("div");
    lobbyElement.className = "lobby-item";
    lobbyElement.innerHTML = `
            <div class="lobby-info">
                <span>${lobby.name || "Unnamed Lobby"}</span>
                <span>${lobby.playerCount} players</span>
            </div>
            <button onclick="showJoinForm('${lobby.id}', '${lobby.name || "Unnamed Lobby"}')">
                Join
            </button>
        `;
    lobbyList.appendChild(lobbyElement);
  });
}

document.addEventListener("DOMContentLoaded", initializeApp);

setInterval(initializeApp, 10000);
