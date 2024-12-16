let socket = null;
let playerName = '';
let currentLobbyId = null;
let roundStartTime;
let roundDuration;
let timerInterval;
let hasAnswered = false;
let playerAnswer = null;
let colors = [];
let answeredPlayers = [];
let totalPlayers = 0;
// const wsProtocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
// const wsUrl = `${wsProtocol}//${window.location.host}/ws?lobby=${lobbyId}`;
// socket = new WebSocket(wsUrl);

// Lobby Management Functions
async function createLobby() {
    const lobbyName = document.getElementById('newLobbyName').value;
    try {
        const response = await fetch('/api/lobbies', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({
                name: lobbyName || null // Send null if no name provided
            }),
        });

        if (!response.ok) {
            throw new Error('Failed to create lobby');
        }

        const lobby = await response.json();
        showJoinForm(lobby.id, lobby.name || 'Unnamed Lobby');
    } catch (error) {
        console.error('Error creating lobby:', error);
        alert('Failed to create lobby. Please try again.');
    }
}

function showJoinForm(lobbyId, lobbyName) {
    currentLobbyId = lobbyId;
    document.getElementById('lobbySelection').style.display = 'none';
    document.getElementById('selectedLobbyName').textContent = lobbyName;
    document.getElementById('joinForm').style.display = 'block';
}


function connectToLobby(lobbyId) {
    if (socket) {
        socket.close();
    }

    const wsProtocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsUrl = `${wsProtocol}//${window.location.host}/ws?lobby=${lobbyId}`;
    socket = new WebSocket(wsUrl);

    socket.onopen = function() {
        socket.send(JSON.stringify({
            action: 'join',
            name: playerName
        }));
        document.getElementById('joinForm').style.display = 'none';
        document.getElementById('lobbyInfo').style.display = 'block';
    };

    setupWebSocketHandlers();
}

function setupWebSocketHandlers() {
    socket.onmessage = function(event) {
        const data = JSON.parse(event.data);
        switch (data.action) {
            case 'game_state':
                handleGameState(data);
                break;
            case 'color_result':
                handleColorResult(data);
                break;
            case 'update_answer_count':
                updateAnswerStatus(data.answered_count, data.total_players);
                break;
            case 'player_answered':
                handlePlayerAnswer(data.player_name, data.correct);
                break;
            case 'error':
                handleError(data.message);
                break;
        }
    };

    socket.onclose = function(event) {
        console.log('WebSocket connection closed. Attempting to reconnect...');
        setTimeout(() => {
            if (currentLobbyId && playerName) {
                connectToLobby(currentLobbyId);
                socket.onopen = function() {
                    socket.send(JSON.stringify({
                        action: 'join',
                        name: playerName
                    }));
                };
            }
        }, 3000);
    };

    socket.onerror = function(error) {
        console.error('WebSocket error:', error);
    };
}

function joinLobby() {
    playerName = document.getElementById('playerName').value;
    if (!playerName) {
        alert('Please enter your name');
        return;
    }

    if (!currentLobbyId) {
        alert('No lobby selected');
        return;
    }

    connectToLobby(currentLobbyId);
}

function leaveLobby() {
    if (socket) {
        socket.close();
    }
    currentLobbyId = null;
    playerName = '';
    document.getElementById('lobbyInfo').style.display = 'none';
    document.getElementById('lobbySelection').style.display = 'block';
    resetGameState();
    initializeApp(); // Refresh lobby list
}

function resetGameState() {
    hasAnswered = false;
    playerAnswer = null;
    colors = [];
    answeredPlayers = [];
    totalPlayers = 0;
    stopTimer();
    document.getElementById('colorButtons').innerHTML = '';
    document.getElementById('leaderboard').innerHTML = '';
    document.getElementById('roundResult').textContent = '';
    document.getElementById('yourScore').textContent = '0';
}

function handleGameState(data) {
    document.getElementById('gameState').textContent =
        `Current Phase: ${data.state.charAt(0).toUpperCase() + data.state.slice(1)}`;
    updateYourScore(data.score);

    if (data.state === 'question') {
        hasAnswered = data.hasAnswered;
        playerAnswer = data.answer;
        colors = data.colors;
        answeredPlayers = [];
        totalPlayers = data.total_players;

        createColorButtons(colors);
        document.getElementById('colorButtons').style.display = 'grid';
        document.getElementById('leaderboard').style.display = 'none';
        document.getElementById('roundResult').textContent = '';
        document.getElementById('answerStatusContainer').style.display = 'flex';
        updateAnswerStatus(data.answered_count, data.total_players);

        let timeLeftMs = data.round_time_left || 0;
        startTimer(timeLeftMs);

    } else if (data.state === 'score') {
        document.getElementById('colorButtons').style.display = 'none';
        updateLeaderboard(data.leaderboard);
        document.getElementById('leaderboard').style.display = 'block';
        document.getElementById('answerStatusContainer').style.display = 'none';
        stopTimer();
    }
}

function startTimer(timeLeftMs) {
    stopTimer();
    timerInterval = setInterval(() => {
        timeLeftMs -= 10;
        if (timeLeftMs <= 0) {
            stopTimer();
            document.getElementById('timer').textContent = "Time's up!";
            document.getElementById('colorButtons').style.display = 'none';
        } else {
            const secondsRemaining = (timeLeftMs / 1000).toFixed(2);
            document.getElementById('timer').textContent = `Time remaining: ${secondsRemaining}s`;
        }
    }, 10);
}

function stopTimer() {
    clearInterval(timerInterval);
    document.getElementById('timer').textContent = '';
}

function updateYourScore(score) {
    document.getElementById('yourScore').textContent = score;
}

function createColorButtons(colors) {
    const colorButtons = document.getElementById('colorButtons');
    colorButtons.innerHTML = '';
    colors.forEach(color => {
        const button = document.createElement('button');
        button.className = 'color-button';
        button.style.backgroundColor = color.rgb;
        button.onclick = () => selectColor(color.name);
        button.title = color.name;
        if (hasAnswered && playerAnswer === color.name) {
            button.style.border = '3px solid white';
        }
        if (hasAnswered) {
            button.disabled = true;
        }
        colorButtons.appendChild(button);
    });
}

function selectColor(colorName) {
    if (!hasAnswered) {
        socket.send(JSON.stringify({ action: 'select_color', color: colorName }));
        hasAnswered = true;
        playerAnswer = colorName;
        createColorButtons(colors);  // Redraw buttons to show selection
    }
}

function handleColorResult(data) {
    updateYourScore(data.total_score);
    const resultText = data.correct ?
        `Correct! You earned ${data.score} points this round.` :
        `Wrong color. You earned 0 points this round.`;
    document.getElementById('roundResult').textContent = resultText;
}

function updateAnswerStatus(answeredCount, totalPlayers) {
    const counterElement = document.getElementById('answerCounter');
    counterElement.textContent = `${answeredCount}/${totalPlayers}`;

    const answeredPlayersElement = document.getElementById('answeredPlayers');
    answeredPlayersElement.innerHTML = '';

    answeredPlayers.forEach(p => {
        const playerSpan = document.createElement('span');
        playerSpan.className = 'answered-player';
        playerSpan.style.backgroundColor = p.correct ? '#35cf0e' : '#cf0e22';
        playerSpan.textContent = p.playerName;
        answeredPlayersElement.appendChild(playerSpan);
    });
}

function handlePlayerAnswer(playerName, correct) {
    if (!answeredPlayers.some(p => p.playerName === playerName)) {
        answeredPlayers.push({ playerName, correct });
        updateAnswerStatus(answeredPlayers.length, totalPlayers);
    }
}

function updateLeaderboard(players) {
    const leaderboard = document.getElementById('leaderboard');
    leaderboard.innerHTML = '<h2>Leaderboard</h2>';
    players.sort((a, b) => b.score - a.score);
    const maxScore = players.length > 0 ? players[0].score : 0;

    players.forEach((player, index) => {
        const playerItem = document.createElement('div');
        playerItem.className = 'player-item';

        const playerInfo = document.createElement('div');
        playerInfo.className = 'player-info';
        playerInfo.innerHTML = `<span>${index + 1}. ${player.name}</span><span>${player.score} points</span>`;

        const progressBar = document.createElement('div');
        progressBar.className = 'progress-bar';

        const progress = document.createElement('div');
        progress.className = 'progress';
        const width = maxScore > 0 ? (player.score / maxScore) * 100 : 0;
        progress.style.width = `${width}%`;

        progressBar.appendChild(progress);
        playerItem.appendChild(playerInfo);
        playerItem.appendChild(progressBar);
        leaderboard.appendChild(playerItem);
    });
}

function handleError(message) {
    console.error('Error:', message);
    alert(message);
}

async function initializeApp() {
    try {
        const response = await fetch('/api/lobbies');
        if (!response.ok) {
            throw new Error('Failed to fetch lobbies');
        }
        const lobbies = await response.json();
        updateLobbyList(lobbies);
    } catch (error) {
        console.error('Error fetching lobbies:', error);
        document.getElementById('lobbyList').innerHTML =
            '<p class="error-message">Failed to load lobbies. Please try again later.</p>';
    }
}

function updateLobbyList(lobbies) {
    const lobbyList = document.getElementById('lobbyList');
    lobbyList.innerHTML = '';

    if (lobbies.length === 0) {
        lobbyList.innerHTML = '<p>No active lobbies. Create one to get started!</p>';
        return;
    }

    lobbies.forEach(lobby => {
        const lobbyElement = document.createElement('div');
        lobbyElement.className = 'lobby-item';
        lobbyElement.innerHTML = `
            <div class="lobby-info">
                <span>${lobby.name || 'Unnamed Lobby'}</span>
                <span>${lobby.player_count} players</span>
            </div>
            <button onclick="showJoinForm('${lobby.id}', '${lobby.name || 'Unnamed Lobby'}')">
                Join
            </button>
        `;
        lobbyList.appendChild(lobbyElement);
    });
}

// Start the application
document.addEventListener('DOMContentLoaded', initializeApp);

// Refresh lobby list periodically
setInterval(initializeApp, 10000);
