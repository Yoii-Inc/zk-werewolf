// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "./DeployHelpers.s.sol";
import "../contracts/WerewolfGame.sol";
import "../contracts/WerewolfProofVerifier.sol";
import "../contracts/WerewolfRewards.sol";
import "../contracts/verifiers/RoleAssignmentGroth16Verifier.sol";
import "../contracts/verifiers/RoleAssignmentGroth16VerifierAdapter.sol";

contract DeployWerewolf is ScaffoldETHDeploy {
    function run() external ScaffoldEthDeployerRunner {
        WerewolfGame game = new WerewolfGame();
        WerewolfProofVerifier verifier = new WerewolfProofVerifier();
        WerewolfRewards rewards = new WerewolfRewards(address(game));
        RoleAssignmentGroth16Verifier roleAssignmentVerifier = new RoleAssignmentGroth16Verifier();
        RoleAssignmentGroth16VerifierAdapter roleAssignmentAdapter =
            new RoleAssignmentGroth16VerifierAdapter(address(roleAssignmentVerifier));

        game.setVerifier(address(verifier));
        game.setRewardsContract(address(rewards));
        verifier.setGameContract(address(game));
        verifier.setRoleAssignmentVerifierAdapter(address(roleAssignmentAdapter));
        rewards.setGameContract(address(game));
    }
}
