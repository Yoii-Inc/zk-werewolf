// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "./DeployHelpers.s.sol";
import "../contracts/WerewolfGame.sol";
import "../contracts/WerewolfProofVerifier.sol";
import "../contracts/WerewolfRewards.sol";
import "../contracts/verifiers/AnonymousVotingGroth16Verifier.sol";
import "../contracts/verifiers/AnonymousVotingGroth16VerifierAdapter.sol";
import "../contracts/verifiers/DivinationGroth16Verifier.sol";
import "../contracts/verifiers/DivinationGroth16VerifierAdapter.sol";
import "../contracts/verifiers/KeyPublicizeGroth16Verifier.sol";
import "../contracts/verifiers/KeyPublicizeGroth16VerifierAdapter.sol";
import "../contracts/verifiers/RoleAssignmentGroth16Verifier.sol";
import "../contracts/verifiers/RoleAssignmentGroth16VerifierAdapter.sol";
import "../contracts/verifiers/WinningJudgementGroth16Verifier.sol";
import "../contracts/verifiers/WinningJudgementGroth16VerifierAdapter.sol";

contract DeployWerewolf is ScaffoldETHDeploy {
    function run() external ScaffoldEthDeployerRunner {
        WerewolfGame game = new WerewolfGame();
        WerewolfProofVerifier verifier = new WerewolfProofVerifier();
        WerewolfRewards rewards = new WerewolfRewards(address(game));

        RoleAssignmentGroth16Verifier roleAssignmentVerifier = new RoleAssignmentGroth16Verifier();
        RoleAssignmentGroth16VerifierAdapter roleAssignmentAdapter =
            new RoleAssignmentGroth16VerifierAdapter(address(roleAssignmentVerifier));
        DivinationGroth16Verifier divinationVerifier = new DivinationGroth16Verifier();
        DivinationGroth16VerifierAdapter divinationAdapter =
            new DivinationGroth16VerifierAdapter(address(divinationVerifier));
        AnonymousVotingGroth16Verifier anonymousVotingVerifier = new AnonymousVotingGroth16Verifier();
        AnonymousVotingGroth16VerifierAdapter anonymousVotingAdapter =
            new AnonymousVotingGroth16VerifierAdapter(address(anonymousVotingVerifier));
        WinningJudgementGroth16Verifier winningJudgementVerifier = new WinningJudgementGroth16Verifier();
        WinningJudgementGroth16VerifierAdapter winningJudgementAdapter =
            new WinningJudgementGroth16VerifierAdapter(address(winningJudgementVerifier));
        KeyPublicizeGroth16Verifier keyPublicizeVerifier = new KeyPublicizeGroth16Verifier();
        KeyPublicizeGroth16VerifierAdapter keyPublicizeAdapter =
            new KeyPublicizeGroth16VerifierAdapter(address(keyPublicizeVerifier));

        game.setVerifier(address(verifier));
        game.setRewardsContract(address(rewards));
        verifier.setGameContract(address(game));

        verifier.setRoleAssignmentVerifierAdapter(address(roleAssignmentAdapter));
        verifier.setDivinationVerifierAdapter(address(divinationAdapter));
        verifier.setAnonymousVotingVerifierAdapter(address(anonymousVotingAdapter));
        verifier.setWinningJudgementVerifierAdapter(address(winningJudgementAdapter));
        verifier.setKeyPublicizeVerifierAdapter(address(keyPublicizeAdapter));

        rewards.setGameContract(address(game));
    }
}
