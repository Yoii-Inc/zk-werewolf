// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "openzeppelin-contracts/contracts/access/Ownable.sol";

interface IVerifierAdapter {
    function verify(bytes calldata proof, bytes calldata publicInputs) external view returns (bool);
}

contract WerewolfProofVerifier is Ownable {
    enum ProofType {
        RoleAssignment,
        Divination,
        AnonymousVoting,
        WinningJudgement,
        KeyPublicize
    }

    struct ProofRecord {
        ProofType proofType;
        bytes32 gameId;
        bytes32 proofHash;
        bool verified;
        uint256 timestamp;
    }

    mapping(bytes32 => ProofRecord) public proofs;
    mapping(bytes32 => mapping(ProofType => bool)) private verifiedByType;
    mapping(bytes32 => address) public verifierAdapterByCircuit;

    address public gameContract;

    event ProofVerified(bytes32 indexed proofId, bytes32 indexed gameId, ProofType proofType, uint256 timestamp);
    event ProofFailed(bytes32 indexed proofId, bytes32 indexed gameId, ProofType proofType, string reason);
    event VerifierAdapterSet(bytes32 indexed circuitKey, ProofType proofType, uint8 playerCount, uint8 werewolfCount, address adapter);

    modifier onlyGameOrOwner() {
        require(msg.sender == gameContract || msg.sender == owner(), "Not authorized");
        _;
    }

    constructor() Ownable(msg.sender) { }

    function setGameContract(address _gameContract) external onlyOwner {
        require(_gameContract != address(0), "Invalid game contract");
        gameContract = _gameContract;
    }

    function buildCircuitKey(ProofType proofType, uint8 playerCount, uint8 werewolfCount) public pure returns (bytes32) {
        return keccak256(abi.encodePacked(uint8(proofType), playerCount, werewolfCount));
    }

    function setVerifierAdapter(ProofType proofType, uint8 playerCount, uint8 werewolfCount, address adapter) external onlyOwner {
        require(adapter != address(0), "Invalid adapter");
        require(isSupportedProfile(proofType, playerCount, werewolfCount), "Unsupported proof profile");

        bytes32 key = buildCircuitKey(proofType, playerCount, werewolfCount);
        verifierAdapterByCircuit[key] = adapter;

        emit VerifierAdapterSet(key, proofType, playerCount, werewolfCount, adapter);
    }

    function getVerifierAdapter(ProofType proofType, uint8 playerCount, uint8 werewolfCount) external view returns (address) {
        bytes32 key = buildCircuitKey(proofType, playerCount, werewolfCount);
        return verifierAdapterByCircuit[key];
    }

    function verifyProof(
        bytes32 proofId,
        bytes32 gameId,
        ProofType proofType,
        uint8 playerCount,
        uint8 werewolfCount,
        bytes calldata proof,
        bytes calldata publicInputs
    ) external onlyGameOrOwner returns (bool) {
        if (!isSupportedProfile(proofType, playerCount, werewolfCount)) {
            emit ProofFailed(proofId, gameId, proofType, "Unsupported proof profile");
            return false;
        }

        if (proof.length == 0) {
            emit ProofFailed(proofId, gameId, proofType, "Empty proof");
            return false;
        }

        address adapter = verifierAdapterByCircuit[buildCircuitKey(proofType, playerCount, werewolfCount)];

        if (adapter == address(0)) {
            emit ProofFailed(proofId, gameId, proofType, "Adapter not set for profile");
            return false;
        }

        bool verified;
        try IVerifierAdapter(adapter).verify(proof, publicInputs) returns (bool ok) {
            verified = ok;
        } catch Error(string memory reason) {
            emit ProofFailed(proofId, gameId, proofType, reason);
            return false;
        } catch {
            emit ProofFailed(proofId, gameId, proofType, "Verification reverted");
            return false;
        }

        if (!verified) {
            emit ProofFailed(proofId, gameId, proofType, "Proof invalid");
            return false;
        }

        proofs[proofId] = ProofRecord({
            proofType: proofType,
            gameId: gameId,
            proofHash: keccak256(abi.encodePacked(proof, publicInputs)),
            verified: true,
            timestamp: block.timestamp
        });
        verifiedByType[gameId][proofType] = true;

        emit ProofVerified(proofId, gameId, proofType, block.timestamp);
        return true;
    }

    function getProofRecord(bytes32 proofId) external view returns (ProofRecord memory) {
        return proofs[proofId];
    }

    function isProofVerified(bytes32 gameId, ProofType proofType) external view returns (bool) {
        return verifiedByType[gameId][proofType];
    }

    function isSupportedProfile(ProofType proofType, uint8 playerCount, uint8 werewolfCount) public pure returns (bool) {
        if (proofType == ProofType.RoleAssignment) {
            if (playerCount == 4) return werewolfCount == 1;
            if (playerCount == 5) return werewolfCount == 1 || werewolfCount == 2;
            if (playerCount == 6) return werewolfCount == 1 || werewolfCount == 2;
            if (playerCount == 7) return werewolfCount >= 1 && werewolfCount <= 3;
            if (playerCount == 8) return werewolfCount >= 1 && werewolfCount <= 3;
            if (playerCount == 9) return werewolfCount >= 1 && werewolfCount <= 3;
            return false;
        }

        return playerCount >= 4 && playerCount <= 9 && werewolfCount == 0;
    }
}
