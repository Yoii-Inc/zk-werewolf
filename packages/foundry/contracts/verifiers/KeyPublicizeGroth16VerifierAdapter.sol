// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "./KeyPublicizeGroth16Verifier.sol";

contract KeyPublicizeGroth16VerifierAdapter {
    KeyPublicizeGroth16Verifier public immutable verifier;

    constructor(address verifierAddress) {
        require(verifierAddress != address(0), "Invalid verifier");
        verifier = KeyPublicizeGroth16Verifier(verifierAddress);
    }

    function verify(bytes calldata proofBytes, bytes calldata publicInputBytes) external view returns (bool) {
        require(publicInputBytes.length == 0, "KeyPublicize expects empty public inputs");
        KeyPublicizeGroth16Verifier.Proof memory proof = abi.decode(proofBytes, (KeyPublicizeGroth16Verifier.Proof));
        return verifier.verifyTx(proof);
    }
}
