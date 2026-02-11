// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import "../contracts/WerewolfGame.sol";
import "../contracts/WerewolfProofVerifier.sol";
import "../contracts/WerewolfRewards.sol";

contract WerewolfGameTest is Test {
    WerewolfGame public game;
    WerewolfProofVerifier public verifier;
    WerewolfRewards public rewards;

    address internal player1 = address(0x1);
    address internal player2 = address(0x2);
    address internal outsider = address(0x99);

    event GameCreated(bytes32 indexed gameId, address[] players, uint256 timestamp);
    event CommitmentSubmitted(bytes32 indexed gameId, address indexed player, bytes32 commitment);

    function setUp() public {
        game = new WerewolfGame();
        verifier = new WerewolfProofVerifier();
        rewards = new WerewolfRewards(address(game));

        game.setVerifier(address(verifier));
        game.setRewardsContract(address(rewards));
        verifier.setGameContract(address(game));
        rewards.setGameContract(address(game));
    }

    function testCreateGameStoresStatePlayersAndRulesHash() public {
        bytes32 gameId = keccak256("test-game-1");
        bytes32 rulesHash = keccak256("rules-v1");
        address[] memory players = new address[](2);
        players[0] = player1;
        players[1] = player2;

        vm.expectEmit(true, false, false, true);
        emit GameCreated(gameId, players, block.timestamp);
        game.createGame(gameId, players, rulesHash);

        (bytes32 stateHash, uint256 startTime,, WerewolfGame.GameStatus status,) = game.games(gameId);
        assertEq(stateHash, bytes32(0));
        assertGt(startTime, 0);
        assertEq(uint8(status), uint8(WerewolfGame.GameStatus.Waiting));
        assertEq(game.gameRulesHashes(gameId), rulesHash);

        address[] memory storedPlayers = game.getGamePlayers(gameId);
        assertEq(storedPlayers.length, 2);
        assertEq(storedPlayers[0], player1);
        assertEq(storedPlayers[1], player2);

        assertTrue(game.isGamePlayer(gameId, player1));
        assertTrue(game.isGamePlayer(gameId, player2));
    }

    function testCreateGameRevertsWhenDuplicatedPlayerExists() public {
        bytes32 gameId = keccak256("test-game-dup");
        address[] memory players = new address[](2);
        players[0] = player1;
        players[1] = player1;

        vm.expectRevert(bytes("Duplicate player"));
        game.createGame(gameId, players);
    }

    function testCreateGameRevertsWhenGameAlreadyExists() public {
        bytes32 gameId = keccak256("test-game-existing");
        address[] memory players = _twoPlayers();
        game.createGame(gameId, players);

        vm.expectRevert(bytes("Game already exists"));
        game.createGame(gameId, players);
    }

    function testSubmitCommitmentStoresCommitmentAndIncrementsCount() public {
        bytes32 gameId = keccak256("test-game-2");
        bytes32 commitment = keccak256("player1-commitment");
        game.createGame(gameId, _twoPlayers());

        vm.prank(player1);
        vm.expectEmit(true, true, false, true);
        emit CommitmentSubmitted(gameId, player1, commitment);
        game.submitCommitment(gameId, commitment);

        (bytes32 storedCommitment, uint256 timestamp) = game.commitments(gameId, player1);
        assertEq(storedCommitment, commitment);
        assertGt(timestamp, 0);
        assertEq(game.commitmentCounts(gameId), 1);
        assertTrue(game.hasSubmittedCommitment(gameId, player1));
    }

    function testSubmitCommitmentRevertsForNonPlayer() public {
        bytes32 gameId = keccak256("test-game-outsider");
        game.createGame(gameId, _twoPlayers());

        vm.prank(outsider);
        vm.expectRevert(bytes("Not a player"));
        game.submitCommitment(gameId, keccak256("outsider-commit"));
    }

    function testSubmitCommitmentRevertsForZeroCommitment() public {
        bytes32 gameId = keccak256("test-game-zero");
        game.createGame(gameId, _twoPlayers());

        vm.prank(player1);
        vm.expectRevert(bytes("Invalid commitment"));
        game.submitCommitment(gameId, bytes32(0));
    }

    function testSubmitCommitmentRevertsForDuplicateSubmission() public {
        bytes32 gameId = keccak256("test-game-duplicate-commit");
        game.createGame(gameId, _twoPlayers());

        vm.startPrank(player1);
        game.submitCommitment(gameId, keccak256("first-commit"));
        vm.expectRevert(bytes("Commitment already submitted"));
        game.submitCommitment(gameId, keccak256("second-commit"));
        vm.stopPrank();
    }

    function testSubmitCommitmentRevertsAfterGameStarted() public {
        bytes32 gameId = keccak256("test-game-started");
        game.createGame(gameId, _twoPlayers());
        game.updateGameState(gameId, keccak256("state-1"));

        vm.prank(player1);
        vm.expectRevert(bytes("Commit phase ended"));
        game.submitCommitment(gameId, keccak256("late-commit"));
    }

    function testUpdateAndFinalizeGame() public {
        bytes32 gameId = keccak256("test-game-3");
        game.createGame(gameId, _twoPlayers());

        bytes32 stateHash = keccak256("state-1");
        game.updateGameState(gameId, stateHash);
        assertTrue(game.verifyGameState(gameId, stateHash));

        game.finalizeGame(gameId, WerewolfGame.GameResult.VillagerWin);

        (, , uint256 endTime, WerewolfGame.GameStatus status, WerewolfGame.GameResult result) = game.games(gameId);
        assertGt(endTime, 0);
        assertEq(uint8(status), uint8(WerewolfGame.GameStatus.Finished));
        assertEq(uint8(result), uint8(WerewolfGame.GameResult.VillagerWin));
    }

    function testRewardDistributionAndClaim() public {
        bytes32 gameId = keccak256("test-game-4");

        rewards.setGamePool(gameId, 0.1 ether, 10000);
        rewards.depositReward{ value: 1 ether }(gameId);

        address[] memory winners = new address[](2);
        winners[0] = player1;
        winners[1] = player2;

        vm.prank(address(game));
        rewards.distributeRewards(gameId, winners);

        uint256 claimable = rewards.claimableRewards(gameId, player1);
        assertEq(claimable, 0.5 ether);

        uint256 beforeBalance = player1.balance;
        vm.prank(player1);
        rewards.claimReward(gameId);
        assertEq(player1.balance, beforeBalance + 0.5 ether);
    }

    function _twoPlayers() internal view returns (address[] memory players) {
        players = new address[](2);
        players[0] = player1;
        players[1] = player2;
    }
}
