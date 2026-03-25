// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import "../contracts/WerewolfGame.sol";
import "../contracts/WerewolfProofVerifier.sol";
import "../contracts/verifiers/Groth16VerifierAdapter.sol";
import "../contracts/verifiers/generated/RoleAssignmentN5W1Groth16Verifier.sol";

contract RoleAssignmentProofVerificationTest is Test {
    uint256 internal constant FIXED_PUBLIC_INPUTS = 100; // (5 players + 5 groups)^2

    WerewolfGame internal game;
    WerewolfProofVerifier internal verifier;
    RoleAssignmentN5W1Groth16Verifier internal roleAssignmentGroth16Verifier;
    Groth16VerifierAdapter internal roleAssignmentGroth16Adapter;

    function setUp() public {
        game = new WerewolfGame();
        verifier = new WerewolfProofVerifier();
        roleAssignmentGroth16Verifier = new RoleAssignmentN5W1Groth16Verifier();
        roleAssignmentGroth16Adapter = new Groth16VerifierAdapter(
            address(roleAssignmentGroth16Verifier),
            bytes4(
                keccak256(
                    "verifyTx(((uint256,uint256),(uint256[2],uint256[2]),(uint256,uint256)),uint256[100])"
                )
            ),
            FIXED_PUBLIC_INPUTS
        );

        game.setVerifier(address(verifier));
        verifier.setGameContract(address(game));
        verifier.setVerifierAdapter(WerewolfProofVerifier.ProofType.RoleAssignment, 5, 1, address(roleAssignmentGroth16Adapter));
    }

    function testRoleAssignmentSingleProverProofCanBeVerifiedOnContract() public {
        bool runFfi = vm.envOr("RUN_ROLE_ASSIGNMENT_GROTH16_FFI_TEST", vm.envOr("RUN_ROLE_ASSIGNMENT_FFI_TEST", false));
        if (!runFfi) {
            emit log("skip: set RUN_ROLE_ASSIGNMENT_GROTH16_FFI_TEST=true and run forge test --ffi");
            return;
        }

        bytes32 gameId = keccak256("role-assignment-game");
        bytes32 proofId = keccak256("role-assignment-proof");
        bytes32 rulesHash = keccak256("rules-v1");

        address[] memory players = new address[](5);
        players[0] = address(0x1);
        players[1] = address(0x2);
        players[2] = address(0x3);
        players[3] = address(0x4);
        players[4] = address(0x5);
        game.createGame(gameId, players, rulesHash);

        (bytes memory proof, bytes memory publicInputs, bool offchainVerified) = _generateRoleAssignmentFixture();
        assertGt(proof.length, 0, "proof should not be empty");
        assertGt(publicInputs.length, 0, "public inputs should not be empty");
        assertTrue(offchainVerified, "offchain Groth16 verification should be true");

        bool ok = game.verifyProofAndRecord(
            proofId, gameId, IWerewolfProofVerifier.ProofType.RoleAssignment, 5, 1, proof, publicInputs
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

    function _generateRoleAssignmentFixture()
        internal
        returns (bytes memory proof, bytes memory publicInputs, bool offchainVerified)
    {
        string memory manifestPath = string.concat(vm.projectRoot(), "/../arkworks-solidity-verifier/Cargo.toml");
        string[] memory ffiCmd = new string[](11);
        ffiCmd[0] = "env";
        ffiCmd[1] = "RUSTFLAGS=-Awarnings";
        ffiCmd[2] = "cargo";
        ffiCmd[3] = "run";
        ffiCmd[4] = "--offline";
        ffiCmd[5] = "--locked";
        ffiCmd[6] = "--quiet";
        ffiCmd[7] = "--manifest-path";
        ffiCmd[8] = manifestPath;
        ffiCmd[9] = "--bin";
        ffiCmd[10] = "role_assignment_groth16_fixture";

        string memory json = string(vm.ffi(ffiCmd));

        RoleAssignmentN5W1Groth16Verifier.Proof memory solidityProof;
        solidityProof.a = RoleAssignmentN5W1Groth16VerifierPairing.G1Point(
            uint256(abi.decode(vm.parseJson(json, ".ax"), (bytes32))),
            uint256(abi.decode(vm.parseJson(json, ".ay"), (bytes32)))
        );
        bytes32[] memory bx = abi.decode(vm.parseJson(json, ".bx"), (bytes32[]));
        bytes32[] memory by = abi.decode(vm.parseJson(json, ".by"), (bytes32[]));
        require(bx.length == 2 && by.length == 2, "invalid Groth16 G2 proof shape");
        solidityProof.b = RoleAssignmentN5W1Groth16VerifierPairing.G2Point(
            [uint256(bx[0]), uint256(bx[1])], [uint256(by[0]), uint256(by[1])]
        );
        solidityProof.c = RoleAssignmentN5W1Groth16VerifierPairing.G1Point(
            uint256(abi.decode(vm.parseJson(json, ".cx"), (bytes32))),
            uint256(abi.decode(vm.parseJson(json, ".cy"), (bytes32)))
        );

        proof = abi.encode(solidityProof);

        uint256[FIXED_PUBLIC_INPUTS] memory parsedInputs;
        bytes32[] memory parsedRawInputs = abi.decode(vm.parseJson(json, ".publicInputs"), (bytes32[]));
        require(parsedRawInputs.length == FIXED_PUBLIC_INPUTS, "invalid public input length");
        for (uint256 i = 0; i < FIXED_PUBLIC_INPUTS; i++) {
            parsedInputs[i] = uint256(parsedRawInputs[i]);
        }
        publicInputs = abi.encode(parsedInputs);
        offchainVerified = abi.decode(vm.parseJson(json, ".offchainVerified"), (bool));
    }
}
