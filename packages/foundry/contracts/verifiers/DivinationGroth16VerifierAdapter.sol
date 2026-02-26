// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "./DivinationGroth16Verifier.sol";

contract DivinationGroth16VerifierAdapter {
    DivinationGroth16Verifier public immutable verifier;

    constructor(address verifierAddress) {
        require(verifierAddress != address(0), "Invalid verifier");
        verifier = DivinationGroth16Verifier(verifierAddress);
    }

    function verify(bytes calldata proofBytes, bytes calldata publicInputBytes) external view returns (bool) {
        DivinationGroth16Verifier.Proof memory proof = abi.decode(proofBytes, (DivinationGroth16Verifier.Proof));
        uint256[8] memory publicInputs = abi.decode(publicInputBytes, (uint256[8]));
        return verifier.verifyTx(proof, publicInputs);
    }
}
