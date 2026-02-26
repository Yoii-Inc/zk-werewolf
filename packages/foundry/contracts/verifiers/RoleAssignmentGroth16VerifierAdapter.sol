// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "./RoleAssignmentGroth16Verifier.sol";

contract RoleAssignmentGroth16VerifierAdapter {
    RoleAssignmentGroth16Verifier public immutable verifier;

    constructor(address verifierAddress) {
        require(verifierAddress != address(0), "Invalid verifier");
        verifier = RoleAssignmentGroth16Verifier(verifierAddress);
    }

    function verify(bytes calldata proofBytes, bytes calldata publicInputBytes) external view returns (bool) {
        RoleAssignmentGroth16Verifier.Proof memory proof = abi.decode(proofBytes, (RoleAssignmentGroth16Verifier.Proof));
        uint256[100] memory publicInputs = abi.decode(publicInputBytes, (uint256[100]));
        return verifier.verifyTx(proof, publicInputs);
    }
}
