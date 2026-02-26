// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "./AnonymousVotingGroth16Verifier.sol";

contract AnonymousVotingGroth16VerifierAdapter {
    AnonymousVotingGroth16Verifier public immutable verifier;

    constructor(address verifierAddress) {
        require(verifierAddress != address(0), "Invalid verifier");
        verifier = AnonymousVotingGroth16Verifier(verifierAddress);
    }

    function verify(bytes calldata proofBytes, bytes calldata publicInputBytes) external view returns (bool) {
        AnonymousVotingGroth16Verifier.Proof memory proof = abi.decode(proofBytes, (AnonymousVotingGroth16Verifier.Proof));
        uint256[1] memory publicInputs = abi.decode(publicInputBytes, (uint256[1]));
        return verifier.verifyTx(proof, publicInputs);
    }
}
