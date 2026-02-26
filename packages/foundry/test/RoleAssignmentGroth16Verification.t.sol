// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import "../contracts/verifiers/RoleAssignmentGroth16Verifier.sol";

contract RoleAssignmentGroth16VerificationTest is Test {
    uint256 internal constant FIXED_PUBLIC_INPUTS = 100; // (num_players + num_groups)^2 for 5 players: (5 + 5)^2

    RoleAssignmentGroth16Verifier internal verifier;

    function setUp() public {
        verifier = new RoleAssignmentGroth16Verifier();
    }

    function testRoleAssignmentGroth16ProofCanBeVerifiedOnContract() public {
        if (!vm.envOr("RUN_ROLE_ASSIGNMENT_GROTH16_FFI_TEST", false)) {
            emit log("skip: set RUN_ROLE_ASSIGNMENT_GROTH16_FFI_TEST=true and run forge test --ffi");
            return;
        }

        (
            RoleAssignmentGroth16Verifier.Proof memory proof,
            uint256[FIXED_PUBLIC_INPUTS] memory publicInputs,
            bool offchainVerified
        ) = _generateFixture();

        assertTrue(offchainVerified, "offchain Groth16 verification should be true");
        bool onchainVerified = verifier.verifyTx(proof, publicInputs);
        emit log_named_uint("offchainVerified", offchainVerified ? 1 : 0);
        emit log_named_uint("onchainVerified", onchainVerified ? 1 : 0);
        assertTrue(onchainVerified, "onchain Groth16 verification should be true");
    }

    function _generateFixture()
        internal
        returns (
            RoleAssignmentGroth16Verifier.Proof memory proof,
            uint256[FIXED_PUBLIC_INPUTS] memory publicInputs,
            bool offchainVerified
        )
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

        proof.a = Pairing.G1Point(
            uint256(abi.decode(vm.parseJson(json, ".ax"), (bytes32))),
            uint256(abi.decode(vm.parseJson(json, ".ay"), (bytes32)))
        );

        bytes32[] memory bx = abi.decode(vm.parseJson(json, ".bx"), (bytes32[]));
        bytes32[] memory by = abi.decode(vm.parseJson(json, ".by"), (bytes32[]));
        require(bx.length == 2 && by.length == 2, "invalid groth16 g2 proof shape");
        proof.b = Pairing.G2Point([uint256(bx[0]), uint256(bx[1])], [uint256(by[0]), uint256(by[1])]);

        proof.c = Pairing.G1Point(
            uint256(abi.decode(vm.parseJson(json, ".cx"), (bytes32))),
            uint256(abi.decode(vm.parseJson(json, ".cy"), (bytes32)))
        );

        bytes32[] memory parsedInputs = abi.decode(vm.parseJson(json, ".publicInputs"), (bytes32[]));
        require(parsedInputs.length == FIXED_PUBLIC_INPUTS, "invalid public input length");
        for (uint256 i = 0; i < FIXED_PUBLIC_INPUTS; i++) {
            publicInputs[i] = uint256(parsedInputs[i]);
        }

        offchainVerified = abi.decode(vm.parseJson(json, ".offchainVerified"), (bool));
    }
}
