let socket = new WebSocket('ws://192.168.43.46:8765');
let playerName = '';
let roundStartTime;
let roundDuration;
let timerInterval;
let hasAnswered = false;
let playerAnswer = null;
let colors = [];
let answeredPlayers = [];
let totalPlayers = 0;

socket.onmessage = function (event) {
    const data = JSON.parse(event.data);
    if (data.action === 'game_state') {
        handleGameState(data);
    } else if (data.action === 'color_result') {
        handleColorResult(data);
    } else if (data.action === 'update_answer_count') {
        updateAnswerStatus(data.answeredCount, data.totalPlayers);
    } else if (data.action === 'player_answered') {
        handlePlayerAnswer(data.playerName);
    }
};

function joinLobby() {
    playerName = document.getElementById('playerName').value;
    if (playerName) {
        socket.send(JSON.stringify({action: 'join', name: playerName}));
        document.getElementById('joinForm').style.display = 'none';
        document.getElementById('lobbyInfo').style.display = 'block';
    }
}

function handleGameState(data) {
    document.getElementById('gameState').textContent = `Current Phase: ${data.state.charAt(0).toUpperCase() + data.state.slice(1)}`;
    updateYourScore(data.score);

    if (data.state === 'question') {
        roundStartTime = data.roundStartTime * 1000;  // convert to milliseconds
        roundDuration = data.roundDuration * 1000;  // convert to milliseconds
        hasAnswered = data.hasAnswered;
        playerAnswer = data.answer;
        colors = data.colors;
        answeredPlayers = [];  // Reset answered players for new round
        totalPlayers = data.totalPlayers;
        createColorButtons(colors);
        document.getElementById('colorButtons').style.display = 'grid';
        document.getElementById('leaderboard').style.display = 'none';
        document.getElementById('roundResult').textContent = '';
        document.getElementById('answerStatusContainer').style.display = 'flex';
        updateAnswerStatus(data.answeredCount, data.totalPlayers);
        startTimer();
    } else if (data.state === 'score') {
        document.getElementById('colorButtons').style.display = 'none';
        updateLeaderboard(data.leaderboard);
        document.getElementById('leaderboard').style.display = 'block';
        document.getElementById('answerStatusContainer').style.display = 'none';
        stopTimer();
    }
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
        socket.send(JSON.stringify({action: 'select_color', color: colorName}));
        hasAnswered = true;
        playerAnswer = colorName;
        createColorButtons(colors);  // Redraw buttons to show selection
    }
}

function handleColorResult(data) {
    updateYourScore(data.totalScore);
    const resultText = data.correct ?
        `Correct! You earned ${data.score} points this round.` :
        `Wrong color. You earned 0 points this round.`;
    document.getElementById('roundResult').textContent = resultText;
    stopTimer();
}

function updateAnswerStatus(answeredCount, totalPlayers) {
    const counterElement = document.getElementById('answerCounter');
    counterElement.textContent = `${answeredCount}/${totalPlayers}`;
    
    const answeredPlayersElement = document.getElementById('answeredPlayers');
    answeredPlayersElement.innerHTML = '';
    
    answeredPlayers.forEach(player => {
        const playerSpan = document.createElement('span');
        playerSpan.className = 'answered-player';
        playerSpan.textContent = player;
        answeredPlayersElement.appendChild(playerSpan);
    });
}

function handlePlayerAnswer(playerName) {
    if (!answeredPlayers.includes(playerName)) {
        answeredPlayers.push(playerName);
        updateAnswerStatus(answeredPlayers.length, totalPlayers);
    }
}

function updateLeaderboard(players) {
    const leaderboard = document.getElementById('leaderboard');
    leaderboard.innerHTML = '<h2>Leaderboard</h2>';
    players.sort((a, b) => b.score - a.score);
    const maxScore = players[0].score;

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
        const width = (player.score / maxScore) * 100;
        progress.style.width = `${width}%`;

        progressBar.appendChild(progress);
        playerItem.appendChild(playerInfo);
        playerItem.appendChild(progressBar);
        leaderboard.appendChild(playerItem);
    });
}

function startTimer() {
    stopTimer();  // Clear any existing timer
    timerInterval = setInterval(updateTimer, 10);  // Update every 10ms
}

function stopTimer() {
    clearInterval(timerInterval);
    document.getElementById('timer').textContent = '';
}

function updateTimer() {
    const now = Date.now();
    const timeElapsed = now - roundStartTime;
    const timeRemaining = roundDuration - timeElapsed;

    if (timeRemaining <= 0) {
        stopTimer();
        document.getElementById('timer').textContent = 'Time\'s up!';
        document.getElementById('colorButtons').style.display = 'none';
    } else {
        const secondsRemaining = (timeRemaining / 1000).toFixed(2);
        document.getElementById('timer').textContent = `Time remaining: ${secondsRemaining}s`;
    }
}

// Reconnect logic
socket.onclose = function (event) {
    console.log('WebSocket connection closed. Attempting to reconnect...');
    setTimeout(function () {
        socket = new WebSocket('ws://192.168.43.46:8765');
        if (playerName) {
            socket.onopen = function () {
                socket.send(JSON.stringify({action: 'join', name: playerName}));
            };
        }
    }, 3000);
};
