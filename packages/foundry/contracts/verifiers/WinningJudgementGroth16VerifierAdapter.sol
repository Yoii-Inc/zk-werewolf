// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "./WinningJudgementGroth16Verifier.sol";

contract WinningJudgementGroth16VerifierAdapter {
    WinningJudgementGroth16Verifier public immutable verifier;

    constructor(address verifierAddress) {
        require(verifierAddress != address(0), "Invalid verifier");
        verifier = WinningJudgementGroth16Verifier(verifierAddress);
    }

    function verify(bytes calldata proofBytes, bytes calldata publicInputBytes) external view returns (bool) {
        WinningJudgementGroth16Verifier.Proof memory proof = abi.decode(proofBytes, (WinningJudgementGroth16Verifier.Proof));
        uint256[2] memory publicInputs = abi.decode(publicInputBytes, (uint256[2]));
        return verifier.verifyTx(proof, publicInputs);
    }
}
