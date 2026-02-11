// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "./DeployHelpers.s.sol";
import "../contracts/WerewolfGame.sol";
import "../contracts/WerewolfProofVerifier.sol";
import "../contracts/WerewolfRewards.sol";

contract DeployWerewolf is ScaffoldETHDeploy {
    function run() external ScaffoldEthDeployerRunner {
        WerewolfGame game = new WerewolfGame();
        WerewolfProofVerifier verifier = new WerewolfProofVerifier();
        WerewolfRewards rewards = new WerewolfRewards(address(game));

        game.setVerifier(address(verifier));
        game.setRewardsContract(address(rewards));
        verifier.setGameContract(address(game));
        rewards.setGameContract(address(game));
    }
}
