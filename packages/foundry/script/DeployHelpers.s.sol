//SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import { Script } from "forge-std/Script.sol";

contract ScaffoldETHDeploy is Script {
    error InvalidPrivateKey(string);

    struct Deployment {
        string name;
        address addr;
    }

    string root;
    string path;
    Deployment[] public deployments;

    /// @notice The deployer address for every run
    address deployer;

    /// @notice Use this modifier on your run() function on your deploy scripts
    modifier ScaffoldEthDeployerRunner() {
        deployer = _startBroadcast();
        if (deployer == address(0)) {
            revert InvalidPrivateKey("Invalid private key");
        }
        _;
        _stopBroadcast();
        exportDeployments();
    }

    function _startBroadcast() internal returns (address) {
        vm.startBroadcast();
        (, address _deployer,) = vm.readCallers();
        return _deployer;
    }

    function _stopBroadcast() internal {
        vm.stopBroadcast();
    }

    function exportDeployments() internal {
        // fetch already existing contracts
        root = vm.projectRoot();
        path = string.concat(root, "/deployments/");
        string memory chainIdStr = vm.toString(block.chainid);
        path = string.concat(path, string.concat(chainIdStr, ".json"));

        string memory jsonWrite;

        uint256 len = deployments.length;

        for (uint256 i = 0; i < len; i++) {
            vm.serializeString(jsonWrite, vm.toString(deployments[i].addr), deployments[i].name);
        }

        string memory chainName = _resolveChainName(block.chainid);
        jsonWrite = vm.serializeString(jsonWrite, "networkName", chainName);
        vm.writeJson(jsonWrite, path);
    }

    function _resolveChainName(uint256 chainId) internal pure returns (string memory) {
        if (chainId == 31337) return "localhost";
        if (chainId == 1) return "mainnet";
        if (chainId == 11155111) return "sepolia";
        return "unknown";
    }
}
