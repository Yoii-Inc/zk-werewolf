// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "openzeppelin-contracts/contracts/access/Ownable.sol";
import "openzeppelin-contracts/contracts/utils/ReentrancyGuard.sol";

contract WerewolfRewards is Ownable, ReentrancyGuard {
    struct RewardPool {
        uint256 totalAmount;
        uint256 entryFee;
        uint256 winnerShare; // basis points
        bool distributed;
    }

    mapping(bytes32 => RewardPool) public gamePools;
    mapping(bytes32 => mapping(address => bool)) public claimed;
    mapping(bytes32 => mapping(address => uint256)) public claimableRewards;

    address public gameContract;

    event RewardDeposited(bytes32 indexed gameId, address indexed player, uint256 amount);
    event RewardClaimed(bytes32 indexed gameId, address indexed winner, uint256 amount);
    event RewardsDistributed(bytes32 indexed gameId, uint256 totalAmount, uint256 winnerCount);

    modifier onlyGameContract() {
        require(msg.sender == gameContract, "Only game contract");
        _;
    }

    constructor(address _gameContract) Ownable(msg.sender) {
        gameContract = _gameContract;
    }

    function setGameContract(address _gameContract) external onlyOwner {
        require(_gameContract != address(0), "Invalid game contract");
        gameContract = _gameContract;
    }

    function setGamePool(bytes32 gameId, uint256 entryFee, uint256 winnerShare) external onlyOwner {
        require(winnerShare <= 10000, "winnerShare > 100%");
        RewardPool storage pool = gamePools[gameId];
        pool.entryFee = entryFee;
        pool.winnerShare = winnerShare;
    }

    function joinGameWithFee(bytes32 gameId) external payable {
        RewardPool storage pool = gamePools[gameId];
        require(pool.entryFee > 0, "Entry fee not configured");
        require(msg.value >= pool.entryFee, "Insufficient entry fee");

        pool.totalAmount += msg.value;
        emit RewardDeposited(gameId, msg.sender, msg.value);
    }

    function depositReward(bytes32 gameId) external payable {
        require(msg.value > 0, "No reward sent");
        gamePools[gameId].totalAmount += msg.value;
        emit RewardDeposited(gameId, msg.sender, msg.value);
    }

    function distributeRewards(bytes32 gameId, address[] calldata winners) external onlyGameContract {
        RewardPool storage pool = gamePools[gameId];
        require(!pool.distributed, "Already distributed");
        require(pool.totalAmount > 0, "No rewards to distribute");
        require(winners.length > 0, "No winners");

        uint256 distributable = (pool.totalAmount * (pool.winnerShare == 0 ? 10000 : pool.winnerShare)) / 10000;
        uint256 rewardPerWinner = distributable / winners.length;

        require(rewardPerWinner > 0, "Reward per winner is zero");

        for (uint256 i = 0; i < winners.length; i++) {
            claimableRewards[gameId][winners[i]] += rewardPerWinner;
        }

        pool.distributed = true;
        emit RewardsDistributed(gameId, distributable, winners.length);
    }

    function claimReward(bytes32 gameId) external nonReentrant {
        require(!claimed[gameId][msg.sender], "Already claimed");

        uint256 amount = claimableRewards[gameId][msg.sender];
        require(amount > 0, "No reward available");

        claimed[gameId][msg.sender] = true;
        claimableRewards[gameId][msg.sender] = 0;

        (bool success,) = msg.sender.call{ value: amount }("");
        require(success, "Transfer failed");

        emit RewardClaimed(gameId, msg.sender, amount);
    }
}
