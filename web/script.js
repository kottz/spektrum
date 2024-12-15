let socket = new WebSocket('ws://192.168.1.155:8765/ws');
let playerName = '';
let roundStartTime;
let roundDuration;
let timerInterval;
let hasAnswered = false;
let playerAnswer = null;
let colors = [];
let answeredPlayers = [];
let totalPlayers = 0;

socket.onmessage = function(event) {
    const data = JSON.parse(event.data);
    if (data.action === 'game_state') {
        handleGameState(data);
    } else if (data.action === 'color_result') {
        handleColorResult(data);
    } else if (data.action === 'update_answer_count') {
        updateAnswerStatus(data.answeredCount, data.totalPlayers);
    } else if (data.action === 'player_answered') {
        handlePlayerAnswer(data.playerName, data.correct);
    }
};

function joinLobby() {
    playerName = document.getElementById('playerName').value;
    if (playerName) {
        socket.send(JSON.stringify({ action: 'join', name: playerName }));
        document.getElementById('joinForm').style.display = 'none';
        document.getElementById('lobbyInfo').style.display = 'block';
    }
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
        totalPlayers = data.totalPlayers;

        createColorButtons(colors);
        document.getElementById('colorButtons').style.display = 'grid';
        document.getElementById('leaderboard').style.display = 'none';
        document.getElementById('roundResult').textContent = '';
        document.getElementById('answerStatusContainer').style.display = 'flex';
        updateAnswerStatus(data.answeredCount, data.totalPlayers);

        let timeLeftMs = data.roundTimeLeft || 0; // fallback if undefined
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

    answeredPlayers.forEach(p => {
        const playerSpan = document.createElement('span');
        playerSpan.className = 'answered-player';

        if (p.correct) {
            playerSpan.style.backgroundColor = '#35cf0e';
        } else {
            playerSpan.style.backgroundColor = '#cf0e22';
        }
        playerSpan.textContent = p.playerName;

        answeredPlayersElement.appendChild(playerSpan);
    });
}

function handlePlayerAnswer(playerName, correct) {
    if (!answeredPlayers.includes(playerName)) {
        answeredPlayers.push({ playerName, correct });
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


// Reconnect logic
socket.onclose = function(event) {
    console.log('WebSocket connection closed. Attempting to reconnect...');
    setTimeout(function() {
        socket = new WebSocket('ws://192.168.1.155:8765/ws');
        if (playerName) {
            socket.onopen = function() {
                socket.send(JSON.stringify({ action: 'join', name: playerName }));
            };
        }
    }, 3000);
};
