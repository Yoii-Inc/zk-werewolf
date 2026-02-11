// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "openzeppelin-contracts/contracts/access/Ownable.sol";

interface IWerewolfProofVerifier {
    enum ProofType {
        RoleAssignment,
        Divination,
        AnonymousVoting,
        WinningJudgement,
        KeyPublicize
    }

    function verifyProof(
        bytes32 proofId,
        bytes32 gameId,
        ProofType proofType,
        bytes calldata proof,
        bytes calldata publicInputs
    ) external returns (bool);
}

interface IWerewolfRewards {
    function distributeRewards(bytes32 gameId, address[] calldata winners) external;
}

contract WerewolfGame is Ownable {
    enum GameStatus {
        Waiting,
        InProgress,
        Finished
    }

    enum GameResult {
        InProgress,
        VillagerWin,
        WerewolfWin
    }

    struct GameState {
        bytes32 stateHash;
        address[] players;
        uint256 startTime;
        uint256 endTime;
        GameStatus status;
        GameResult result;
    }

    struct PlayerCommitment {
        bytes32 commitment;
        uint256 timestamp;
    }

    mapping(bytes32 => GameState) public games;
    mapping(bytes32 => mapping(address => PlayerCommitment)) public commitments;
    mapping(bytes32 => mapping(address => bool)) public isGamePlayer;
    mapping(bytes32 => bytes32) public gameRulesHashes;
    mapping(bytes32 => uint256) public commitmentCounts;

    address public gameMaster;
    IWerewolfProofVerifier public verifier;
    IWerewolfRewards public rewardsContract;

    event GameCreated(bytes32 indexed gameId, address[] players, uint256 timestamp);
    event CommitmentSubmitted(bytes32 indexed gameId, address indexed player, bytes32 commitment);
    event GameStateUpdated(bytes32 indexed gameId, bytes32 stateHash, uint256 timestamp);
    event GameFinalized(bytes32 indexed gameId, GameResult result, uint256 timestamp);

    modifier onlyGameMasterOrOwner() {
        require(msg.sender == gameMaster || msg.sender == owner(), "Not game master");
        _;
    }

    constructor() Ownable(msg.sender) {
        gameMaster = msg.sender;
    }

    function setGameMaster(address _gameMaster) external onlyOwner {
        require(_gameMaster != address(0), "Invalid game master");
        gameMaster = _gameMaster;
    }

    function setVerifier(address _verifier) external onlyOwner {
        require(_verifier != address(0), "Invalid verifier");
        verifier = IWerewolfProofVerifier(_verifier);
    }

    function setRewardsContract(address _rewards) external onlyOwner {
        require(_rewards != address(0), "Invalid rewards");
        rewardsContract = IWerewolfRewards(_rewards);
    }

    function createGame(bytes32 gameId, address[] calldata players) external onlyGameMasterOrOwner {
        _createGame(gameId, players, bytes32(0));
    }

    function createGame(
        bytes32 gameId,
        address[] calldata players,
        bytes32 rulesHash
    ) external onlyGameMasterOrOwner {
        _createGame(gameId, players, rulesHash);
    }

    function _createGame(bytes32 gameId, address[] calldata players, bytes32 rulesHash) private {
        GameState storage game = games[gameId];
        require(game.startTime == 0, "Game already exists");
        require(players.length > 0, "No players");

        game.stateHash = bytes32(0);
        game.startTime = block.timestamp;
        game.endTime = 0;
        game.status = GameStatus.Waiting;
        game.result = GameResult.InProgress;

        for (uint256 i = 0; i < players.length; i++) {
            require(players[i] != address(0), "Invalid player");
            require(!isGamePlayer[gameId][players[i]], "Duplicate player");
            game.players.push(players[i]);
            isGamePlayer[gameId][players[i]] = true;
        }
        gameRulesHashes[gameId] = rulesHash;

        emit GameCreated(gameId, players, block.timestamp);
    }

    function submitCommitment(bytes32 gameId, bytes32 commitment) external {
        GameState storage game = games[gameId];
        require(game.startTime != 0, "Game not found");
        require(game.status != GameStatus.Finished, "Game finished");
        require(game.status == GameStatus.Waiting, "Commit phase ended");
        require(isGamePlayer[gameId][msg.sender], "Not a player");
        require(commitment != bytes32(0), "Invalid commitment");
        require(commitments[gameId][msg.sender].timestamp == 0, "Commitment already submitted");

        commitments[gameId][msg.sender] = PlayerCommitment({ commitment: commitment, timestamp: block.timestamp });
        commitmentCounts[gameId] += 1;

        emit CommitmentSubmitted(gameId, msg.sender, commitment);
    }

    function updateGameState(bytes32 gameId, bytes32 stateHash) external onlyGameMasterOrOwner {
        GameState storage game = games[gameId];
        require(game.startTime != 0, "Game not found");
        require(game.status != GameStatus.Finished, "Game finished");

        game.stateHash = stateHash;
        if (game.status == GameStatus.Waiting) {
            game.status = GameStatus.InProgress;
        }

        emit GameStateUpdated(gameId, stateHash, block.timestamp);
    }

    function finalizeGame(bytes32 gameId, GameResult result) external onlyGameMasterOrOwner {
        GameState storage game = games[gameId];
        require(game.startTime != 0, "Game not found");
        require(game.status != GameStatus.Finished, "Already finalized");

        game.status = GameStatus.Finished;
        game.result = result;
        game.endTime = block.timestamp;

        emit GameFinalized(gameId, result, block.timestamp);
    }

    function verifyGameState(bytes32 gameId, bytes32 expectedHash) external view returns (bool) {
        return games[gameId].stateHash == expectedHash;
    }

    function getGamePlayers(bytes32 gameId) external view returns (address[] memory) {
        return games[gameId].players;
    }

    function hasSubmittedCommitment(bytes32 gameId, address player) external view returns (bool) {
        return commitments[gameId][player].timestamp != 0;
    }

    function verifyProofAndRecord(
        bytes32 proofId,
        bytes32 gameId,
        IWerewolfProofVerifier.ProofType proofType,
        bytes calldata proof,
        bytes calldata publicInputs
    ) external onlyGameMasterOrOwner returns (bool) {
        require(address(verifier) != address(0), "Verifier not set");
        return verifier.verifyProof(proofId, gameId, proofType, proof, publicInputs);
    }

    function distributeRewards(bytes32 gameId, address[] calldata winners) external onlyGameMasterOrOwner {
        require(address(rewardsContract) != address(0), "Rewards contract not set");
        rewardsContract.distributeRewards(gameId, winners);
    }
}
