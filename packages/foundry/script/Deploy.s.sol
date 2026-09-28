//SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "./DeployHelpers.s.sol";
import { DeployWerewolf } from "./DeployWerewolf.s.sol";

/**
 * @notice Main deployment script for all contracts
 * @dev Run this when you want to deploy multiple contracts at once
 *
 * Example: yarn deploy # runs this script(without`--file` flag)
 */
contract DeployScript is ScaffoldETHDeploy {
    function run() external {
        // Deploys all your contracts sequentially
        // Add new deployments here when needed

        // Note: the scaffold-eth boilerplate DeployYourContract/YourContract is
        // intentionally not deployed here - it isn't part of this app (no real
        // deployment, no reachable page references it) and having it as a
        // second, separate vm.startBroadcast()/stopBroadcast() segment ahead of
        // DeployWerewolf's was tripping forge's nonce tracking under --slow in
        // CI ("EOA nonce changed unexpectedly ... Expected N got N+1").

        DeployWerewolf deployWerewolf = new DeployWerewolf();
        deployWerewolf.run();
    }
}
