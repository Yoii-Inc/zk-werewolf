// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "./DeployHelpers.s.sol";
import "../contracts/WerewolfGame.sol";
import "../contracts/WerewolfProofVerifier.sol";
import "../contracts/WerewolfRewards.sol";
import "../contracts/verifiers/Groth16VerifierAdapter.sol";
import "openzeppelin-contracts/contracts/utils/Strings.sol";

contract DeployWerewolf is ScaffoldETHDeploy {
    using Strings for uint256;

    uint8 internal constant MIN_PLAYERS = 4;
    uint8 internal constant MAX_PLAYERS = 9;

    function run() external ScaffoldEthDeployerRunner {
        WerewolfGame game = new WerewolfGame();
        WerewolfProofVerifier verifier = new WerewolfProofVerifier();
        WerewolfRewards rewards = new WerewolfRewards(address(game));

        deployments.push(Deployment({ name: "WerewolfGame", addr: address(game) }));
        deployments.push(Deployment({ name: "WerewolfProofVerifier", addr: address(verifier) }));
        deployments.push(Deployment({ name: "WerewolfRewards", addr: address(rewards) }));

        game.setVerifier(address(verifier));
        game.setRewardsContract(address(rewards));
        verifier.setGameContract(address(game));
        rewards.setGameContract(address(game));

        _registerRoleAssignmentProfiles(verifier);
        _registerPlayerCountProfiles(verifier);
    }

    function _registerRoleAssignmentProfiles(WerewolfProofVerifier verifier) internal {
        _deployAndRegisterRoleAssignment(verifier, 4, 1);
        _deployAndRegisterRoleAssignment(verifier, 5, 1);
        _deployAndRegisterRoleAssignment(verifier, 5, 2);
        _deployAndRegisterRoleAssignment(verifier, 6, 1);
        _deployAndRegisterRoleAssignment(verifier, 6, 2);
        // Temporarily disabled: exceeds EVM max contract size (24,576 bytes) on Sepolia.
        // _deployAndRegisterRoleAssignment(verifier, 7, 1);
        _deployAndRegisterRoleAssignment(verifier, 7, 2);
        _deployAndRegisterRoleAssignment(verifier, 7, 3);
        // Temporarily disabled: exceeds EVM max contract size (24,576 bytes) on Sepolia.
        // _deployAndRegisterRoleAssignment(verifier, 8, 1);
        // _deployAndRegisterRoleAssignment(verifier, 8, 2);
        // _deployAndRegisterRoleAssignment(verifier, 8, 3);
        // _deployAndRegisterRoleAssignment(verifier, 9, 1);
        // _deployAndRegisterRoleAssignment(verifier, 9, 2);
        // _deployAndRegisterRoleAssignment(verifier, 9, 3);
    }

    function _registerPlayerCountProfiles(WerewolfProofVerifier verifier) internal {
        for (uint8 n = MIN_PLAYERS; n <= MAX_PLAYERS; n++) {
            _deployAndRegister(
                verifier,
                WerewolfProofVerifier.ProofType.Divination,
                n,
                0,
                _divinationContractName(n),
                8
            );
            _deployAndRegister(
                verifier,
                WerewolfProofVerifier.ProofType.AnonymousVoting,
                n,
                0,
                _anonymousVotingContractName(n),
                1
            );
            _deployAndRegister(
                verifier,
                WerewolfProofVerifier.ProofType.WinningJudgement,
                n,
                0,
                _winningJudgementContractName(n),
                2
            );
            _deployAndRegister(
                verifier,
                WerewolfProofVerifier.ProofType.KeyPublicize,
                n,
                0,
                _keyPublicizeContractName(n),
                0
            );
        }
    }

    function _deployAndRegisterRoleAssignment(WerewolfProofVerifier verifier, uint8 playerCount, uint8 werewolfCount)
        internal
    {
        _deployAndRegister(
            verifier,
            WerewolfProofVerifier.ProofType.RoleAssignment,
            playerCount,
            werewolfCount,
            _roleAssignmentContractName(playerCount, werewolfCount),
            _roleAssignmentPublicInputWordLength(playerCount, werewolfCount)
        );
    }

    function _deployAndRegister(
        WerewolfProofVerifier verifier,
        WerewolfProofVerifier.ProofType proofType,
        uint8 playerCount,
        uint8 werewolfCount,
        string memory contractName,
        uint256 publicInputWordLength
    ) internal {
        address deployedVerifier = _deployGeneratedVerifier(contractName);
        bytes4 selector = _verifyTxSelector(publicInputWordLength);
        Groth16VerifierAdapter adapter = new Groth16VerifierAdapter(deployedVerifier, selector, publicInputWordLength);

        verifier.setVerifierAdapter(proofType, playerCount, werewolfCount, address(adapter));

        deployments.push(
            Deployment({
                name: string.concat(contractName, "Adapter_n", uint256(playerCount).toString(), "_w", uint256(werewolfCount).toString()),
                addr: address(adapter)
            })
        );
    }

    function _deployGeneratedVerifier(string memory contractName) internal returns (address deployed) {
        string memory artifactFqn =
            string.concat("contracts/verifiers/generated/", contractName, ".sol:", contractName);
        bytes memory creationCode = vm.getCode(artifactFqn);
        require(creationCode.length != 0, string.concat("Missing generated verifier artifact: ", artifactFqn));

        assembly {
            deployed := create(0, add(creationCode, 0x20), mload(creationCode))
        }
        require(deployed != address(0), string.concat("Failed to deploy verifier: ", contractName));

        deployments.push(Deployment({ name: contractName, addr: deployed }));
    }

    function _verifyTxSelector(uint256 publicInputWordLength) internal pure returns (bytes4) {
        string memory proofTupleType = "((uint256,uint256),(uint256[2],uint256[2]),(uint256,uint256))";
        if (publicInputWordLength == 0) {
            return bytes4(keccak256(bytes(string.concat("verifyTx(", proofTupleType, ")"))));
        }
        return bytes4(
            keccak256(
                bytes(
                    string.concat(
                        "verifyTx(",
                        proofTupleType,
                        ",uint256[",
                        publicInputWordLength.toString(),
                        "])"
                    )
                )
            )
        );
    }

    function _roleAssignmentPublicInputWordLength(uint8 playerCount, uint8 werewolfCount) internal pure returns (uint256) {
        uint256 n = uint256(playerCount);
        uint256 w = uint256(werewolfCount);
        // grouping_parameter(Seer=1, Werewolf=w, Villager=n-w-1) のとき:
        // num_groups = n - w + 1, matrix_size = n + num_groups = 2n - w + 1
        uint256 matrixSize = 2 * n - w + 1;
        return matrixSize * matrixSize;
    }

    function _roleAssignmentContractName(uint8 playerCount, uint8 werewolfCount)
        internal
        pure
        returns (string memory)
    {
        return string.concat(
            "RoleAssignmentN",
            uint256(playerCount).toString(),
            "W",
            uint256(werewolfCount).toString(),
            "Groth16Verifier"
        );
    }

    function _divinationContractName(uint8 playerCount) internal pure returns (string memory) {
        return string.concat("DivinationN", uint256(playerCount).toString(), "Groth16Verifier");
    }

    function _anonymousVotingContractName(uint8 playerCount) internal pure returns (string memory) {
        return string.concat("AnonymousVotingN", uint256(playerCount).toString(), "Groth16Verifier");
    }

    function _winningJudgementContractName(uint8 playerCount) internal pure returns (string memory) {
        return string.concat("WinningJudgementN", uint256(playerCount).toString(), "Groth16Verifier");
    }

    function _keyPublicizeContractName(uint8 playerCount) internal pure returns (string memory) {
        return string.concat("KeyPublicizeN", uint256(playerCount).toString(), "Groth16Verifier");
    }
}
