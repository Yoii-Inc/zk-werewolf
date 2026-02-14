// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import "../contracts/WerewolfGame.sol";
import "../contracts/WerewolfProofVerifier.sol";

contract RoleAssignmentProofVerificationTest is Test {
    WerewolfGame internal game;
    WerewolfProofVerifier internal verifier;

    function setUp() public {
        game = new WerewolfGame();
        verifier = new WerewolfProofVerifier();

        game.setVerifier(address(verifier));
        verifier.setGameContract(address(game));
    }

    function testRoleAssignmentSingleProverProofCanBeVerifiedOnContract() public {
        // Heavy integration path:
        // 1) Generate RoleAssignment proof by single prover (Rust + Marlin)
        // 2) Submit to contract
        // 3) Ensure contract verification path is executed and recorded
        if (!vm.envOr("RUN_ROLE_ASSIGNMENT_FFI_TEST", false)) {
            emit log("skip: set RUN_ROLE_ASSIGNMENT_FFI_TEST=true and run forge test --ffi");
            return;
        }

        bytes32 gameId = keccak256("role-assignment-game");
        bytes32 proofId = keccak256("role-assignment-proof");
        bytes32 rulesHash = keccak256("rules-v1");

        address[] memory players = new address[](4);
        players[0] = address(0x1);
        players[1] = address(0x2);
        players[2] = address(0x3);
        players[3] = address(0x4);
        game.createGame(gameId, players, rulesHash);

        (bytes memory proof, bytes memory publicInputs) = _generateRoleAssignmentFixture();
        assertGt(proof.length, 0, "proof should not be empty");
        assertGt(publicInputs.length, 0, "public inputs should not be empty");

        bool ok = game.verifyProofAndRecord(
            proofId,
            gameId,
            IWerewolfProofVerifier.ProofType.RoleAssignment,
            proof,
            publicInputs
        );

        assertTrue(ok, "verification call should return true");
        assertTrue(
            verifier.isProofVerified(gameId, WerewolfProofVerifier.ProofType.RoleAssignment),
            "proof should be marked verified"
        );

        (
            WerewolfProofVerifier.ProofType recordedType,
            bytes32 recordedGameId,
            bytes32 recordedProofHash,
            bool recordedVerified,
            uint256 recordedTimestamp
        ) = verifier.proofs(proofId);

        assertEq(uint8(recordedType), uint8(WerewolfProofVerifier.ProofType.RoleAssignment));
        assertEq(recordedGameId, gameId);
        assertEq(recordedProofHash, keccak256(abi.encodePacked(proof, publicInputs)));
        assertTrue(recordedVerified);
        assertGt(recordedTimestamp, 0);
    }

    function _generateRoleAssignmentFixture() internal returns (bytes memory proof, bytes memory publicInputs) {
        string memory manifestPath = string.concat(vm.projectRoot(), "/../zk-mpc-node/Cargo.toml");

        string[] memory ffiCmd = new string[](9);
        ffiCmd[0] = "env";
        ffiCmd[1] = "RUSTFLAGS=-Awarnings";
        ffiCmd[2] = "cargo";
        ffiCmd[3] = "run";
        ffiCmd[4] = "--quiet";
        ffiCmd[5] = "--manifest-path";
        ffiCmd[6] = manifestPath;
        ffiCmd[7] = "--bin";
        ffiCmd[8] = "role_assignment_single_prover_fixture";

        bytes memory out = vm.ffi(ffiCmd);
        string memory json = string(out);

        proof = abi.decode(vm.parseJson(json, ".proof"), (bytes));
        publicInputs = abi.decode(vm.parseJson(json, ".publicInputs"), (bytes));
    }
}
