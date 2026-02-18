// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "openzeppelin-contracts/contracts/access/Ownable.sol";

interface IRoleAssignmentVerifierAdapter {
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

    address public gameContract;
    address public roleAssignmentVerifierAdapter;

    event ProofVerified(bytes32 indexed proofId, bytes32 indexed gameId, ProofType proofType, uint256 timestamp);
    event ProofFailed(bytes32 indexed proofId, bytes32 indexed gameId, ProofType proofType, string reason);

    modifier onlyGameOrOwner() {
        require(msg.sender == gameContract || msg.sender == owner(), "Not authorized");
        _;
    }

    constructor() Ownable(msg.sender) { }

    function setGameContract(address _gameContract) external onlyOwner {
        require(_gameContract != address(0), "Invalid game contract");
        gameContract = _gameContract;
    }

    function setRoleAssignmentVerifierAdapter(address _adapter) external onlyOwner {
        require(_adapter != address(0), "Invalid adapter");
        roleAssignmentVerifierAdapter = _adapter;
    }

    function verifyProof(
        bytes32 proofId,
        bytes32 gameId,
        ProofType proofType,
        bytes calldata proof,
        bytes calldata publicInputs
    ) external onlyGameOrOwner returns (bool) {
        if (proof.length == 0 || publicInputs.length == 0) {
            emit ProofFailed(proofId, gameId, proofType, "Empty proof/public inputs");
            return false;
        }

        if (proofType == ProofType.RoleAssignment) {
            if (roleAssignmentVerifierAdapter == address(0)) {
                emit ProofFailed(proofId, gameId, proofType, "RoleAssignment adapter not set");
                return false;
            }

            bool verified;
            try IRoleAssignmentVerifierAdapter(roleAssignmentVerifierAdapter).verify(proof, publicInputs) returns (
                bool ok
            ) {
                verified = ok;
            } catch Error(string memory reason) {
                emit ProofFailed(proofId, gameId, proofType, reason);
                return false;
            } catch {
                emit ProofFailed(proofId, gameId, proofType, "RoleAssignment verification reverted");
                return false;
            }

            if (!verified) {
                emit ProofFailed(proofId, gameId, proofType, "RoleAssignment proof invalid");
                return false;
            }
        }

        bytes32 proofHash = keccak256(abi.encodePacked(proof, publicInputs));

        proofs[proofId] = ProofRecord({
            proofType: proofType, gameId: gameId, proofHash: proofHash, verified: true, timestamp: block.timestamp
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
}
