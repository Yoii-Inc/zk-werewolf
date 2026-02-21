// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import "../contracts/WerewolfGame.sol";
import "../contracts/WerewolfProofVerifier.sol";
import "../contracts/verifiers/AnonymousVotingGroth16Verifier.sol";
import "../contracts/verifiers/AnonymousVotingGroth16VerifierAdapter.sol";
import "../contracts/verifiers/DivinationGroth16Verifier.sol";
import "../contracts/verifiers/DivinationGroth16VerifierAdapter.sol";
import "../contracts/verifiers/KeyPublicizeGroth16Verifier.sol";
import "../contracts/verifiers/KeyPublicizeGroth16VerifierAdapter.sol";
import "../contracts/verifiers/WinningJudgementGroth16Verifier.sol";
import "../contracts/verifiers/WinningJudgementGroth16VerifierAdapter.sol";

contract AdditionalProofVerificationTest is Test {
    uint256 internal constant ANONYMOUS_VOTING_PUBLIC_INPUTS = 1;
    uint256 internal constant DIVINATION_PUBLIC_INPUTS = 8;
    uint256 internal constant WINNING_JUDGEMENT_PUBLIC_INPUTS = 2;

    WerewolfGame internal game;
    WerewolfProofVerifier internal verifier;

    AnonymousVotingGroth16Verifier internal anonymousVotingVerifier;
    AnonymousVotingGroth16VerifierAdapter internal anonymousVotingAdapter;
    DivinationGroth16Verifier internal divinationVerifier;
    DivinationGroth16VerifierAdapter internal divinationAdapter;
    WinningJudgementGroth16Verifier internal winningJudgementVerifier;
    WinningJudgementGroth16VerifierAdapter internal winningJudgementAdapter;
    KeyPublicizeGroth16Verifier internal keyPublicizeVerifier;
    KeyPublicizeGroth16VerifierAdapter internal keyPublicizeAdapter;

    function setUp() public {
        game = new WerewolfGame();
        verifier = new WerewolfProofVerifier();

        anonymousVotingVerifier = new AnonymousVotingGroth16Verifier();
        anonymousVotingAdapter = new AnonymousVotingGroth16VerifierAdapter(address(anonymousVotingVerifier));

        divinationVerifier = new DivinationGroth16Verifier();
        divinationAdapter = new DivinationGroth16VerifierAdapter(address(divinationVerifier));

        winningJudgementVerifier = new WinningJudgementGroth16Verifier();
        winningJudgementAdapter = new WinningJudgementGroth16VerifierAdapter(address(winningJudgementVerifier));

        keyPublicizeVerifier = new KeyPublicizeGroth16Verifier();
        keyPublicizeAdapter = new KeyPublicizeGroth16VerifierAdapter(address(keyPublicizeVerifier));

        game.setVerifier(address(verifier));
        verifier.setGameContract(address(game));
        verifier.setAnonymousVotingVerifierAdapter(address(anonymousVotingAdapter));
        verifier.setDivinationVerifierAdapter(address(divinationAdapter));
        verifier.setWinningJudgementVerifierAdapter(address(winningJudgementAdapter));
        verifier.setKeyPublicizeVerifierAdapter(address(keyPublicizeAdapter));
    }

    function testDivinationProofCanBeVerifiedViaWerewolfVerifier() public {
        bool runFfi = vm.envOr("RUN_DIVINATION_GROTH16_FFI_TEST", vm.envOr("RUN_DIVINATION_FFI_TEST", false));
        if (!runFfi) {
            emit log("skip: set RUN_DIVINATION_GROTH16_FFI_TEST=true and run forge test --ffi");
            return;
        }

        bytes32 gameId = keccak256("divination-game");
        bytes32 proofId = keccak256("divination-proof");
        _createGame(gameId);

        (bytes memory proof, bytes memory publicInputs, bool offchainVerified) = _generateDivinationFixture();
        assertTrue(offchainVerified, "offchain Groth16 verification should be true");

        bool ok = game.verifyProofAndRecord(
            proofId, gameId, IWerewolfProofVerifier.ProofType.Divination, proof, publicInputs
        );

        assertTrue(ok, "verification call should return true");
        assertTrue(
            verifier.isProofVerified(gameId, WerewolfProofVerifier.ProofType.Divination),
            "proof should be marked verified"
        );
    }

    function testAnonymousVotingProofCanBeVerifiedViaWerewolfVerifier() public {
        bool runFfi = vm.envOr(
            "RUN_ANONYMOUS_VOTING_GROTH16_FFI_TEST",
            vm.envOr("RUN_VOTING_GROTH16_FFI_TEST", vm.envOr("RUN_ANONYMOUS_VOTING_FFI_TEST", false))
        );
        if (!runFfi) {
            emit log("skip: set RUN_ANONYMOUS_VOTING_GROTH16_FFI_TEST=true and run forge test --ffi");
            return;
        }

        bytes32 gameId = keccak256("anonymous-voting-game");
        bytes32 proofId = keccak256("anonymous-voting-proof");
        _createGame(gameId);

        (bytes memory proof, bytes memory publicInputs, bool offchainVerified) = _generateAnonymousVotingFixture();
        assertTrue(offchainVerified, "offchain Groth16 verification should be true");

        bool ok = game.verifyProofAndRecord(
            proofId, gameId, IWerewolfProofVerifier.ProofType.AnonymousVoting, proof, publicInputs
        );

        assertTrue(ok, "verification call should return true");
        assertTrue(
            verifier.isProofVerified(gameId, WerewolfProofVerifier.ProofType.AnonymousVoting),
            "proof should be marked verified"
        );
    }

    function testWinningJudgementProofCanBeVerifiedViaWerewolfVerifier() public {
        bool runFfi =
            vm.envOr("RUN_WINNING_JUDGEMENT_GROTH16_FFI_TEST", vm.envOr("RUN_WINNING_JUDGEMENT_FFI_TEST", false));
        if (!runFfi) {
            emit log("skip: set RUN_WINNING_JUDGEMENT_GROTH16_FFI_TEST=true and run forge test --ffi");
            return;
        }

        bytes32 gameId = keccak256("winning-judgement-game");
        bytes32 proofId = keccak256("winning-judgement-proof");
        _createGame(gameId);

        (bytes memory proof, bytes memory publicInputs, bool offchainVerified) = _generateWinningJudgementFixture();
        assertTrue(offchainVerified, "offchain Groth16 verification should be true");

        bool ok = game.verifyProofAndRecord(
            proofId, gameId, IWerewolfProofVerifier.ProofType.WinningJudgement, proof, publicInputs
        );

        assertTrue(ok, "verification call should return true");
        assertTrue(
            verifier.isProofVerified(gameId, WerewolfProofVerifier.ProofType.WinningJudgement),
            "proof should be marked verified"
        );
    }

    function testKeyPublicizeProofCanBeVerifiedViaWerewolfVerifier() public {
        bool runFfi = vm.envOr("RUN_KEY_PUBLICIZE_GROTH16_FFI_TEST", vm.envOr("RUN_KEY_PUBLICIZE_FFI_TEST", false));
        if (!runFfi) {
            emit log("skip: set RUN_KEY_PUBLICIZE_GROTH16_FFI_TEST=true and run forge test --ffi");
            return;
        }

        bytes32 gameId = keccak256("key-publicize-game");
        bytes32 proofId = keccak256("key-publicize-proof");
        _createGame(gameId);

        (bytes memory proof, bytes memory publicInputs, bool offchainVerified) = _generateKeyPublicizeFixture();
        assertTrue(offchainVerified, "offchain Groth16 verification should be true");

        bool ok = game.verifyProofAndRecord(
            proofId, gameId, IWerewolfProofVerifier.ProofType.KeyPublicize, proof, publicInputs
        );

        assertTrue(ok, "verification call should return true");
        assertTrue(
            verifier.isProofVerified(gameId, WerewolfProofVerifier.ProofType.KeyPublicize),
            "proof should be marked verified"
        );
    }

    function _createGame(bytes32 gameId) internal {
        bytes32 rulesHash = keccak256("rules-v1");
        address[] memory players = new address[](5);
        players[0] = address(0x1);
        players[1] = address(0x2);
        players[2] = address(0x3);
        players[3] = address(0x4);
        players[4] = address(0x5);
        game.createGame(gameId, players, rulesHash);
    }

    function _generateAnonymousVotingFixture()
        internal
        returns (bytes memory proof, bytes memory publicInputs, bool offchainVerified)
    {
        string memory json = _runFixture("anonymous_voting_groth16_fixture");

        AnonymousVotingGroth16Verifier.Proof memory solidityProof;
        solidityProof.a = AnonymousVotingPairing.G1Point(
            uint256(abi.decode(vm.parseJson(json, ".ax"), (bytes32))),
            uint256(abi.decode(vm.parseJson(json, ".ay"), (bytes32)))
        );

        bytes32[] memory bx = abi.decode(vm.parseJson(json, ".bx"), (bytes32[]));
        bytes32[] memory by = abi.decode(vm.parseJson(json, ".by"), (bytes32[]));
        require(bx.length == 2 && by.length == 2, "invalid groth16 g2 proof shape");
        solidityProof.b = AnonymousVotingPairing.G2Point([uint256(bx[0]), uint256(bx[1])], [uint256(by[0]), uint256(by[1])]);

        solidityProof.c = AnonymousVotingPairing.G1Point(
            uint256(abi.decode(vm.parseJson(json, ".cx"), (bytes32))),
            uint256(abi.decode(vm.parseJson(json, ".cy"), (bytes32)))
        );

        proof = abi.encode(solidityProof);

        uint256[ANONYMOUS_VOTING_PUBLIC_INPUTS] memory parsedInputs;
        bytes32[] memory parsedRawInputs = abi.decode(vm.parseJson(json, ".publicInputs"), (bytes32[]));
        require(parsedRawInputs.length == ANONYMOUS_VOTING_PUBLIC_INPUTS, "invalid public input length");
        for (uint256 i = 0; i < ANONYMOUS_VOTING_PUBLIC_INPUTS; i++) {
            parsedInputs[i] = uint256(parsedRawInputs[i]);
        }
        publicInputs = abi.encode(parsedInputs);

        offchainVerified = abi.decode(vm.parseJson(json, ".offchainVerified"), (bool));
    }

    function _generateDivinationFixture()
        internal
        returns (bytes memory proof, bytes memory publicInputs, bool offchainVerified)
    {
        string memory json = _runFixture("divination_groth16_fixture");

        DivinationGroth16Verifier.Proof memory solidityProof;
        solidityProof.a = DivinationPairing.G1Point(
            uint256(abi.decode(vm.parseJson(json, ".ax"), (bytes32))),
            uint256(abi.decode(vm.parseJson(json, ".ay"), (bytes32)))
        );

        bytes32[] memory bx = abi.decode(vm.parseJson(json, ".bx"), (bytes32[]));
        bytes32[] memory by = abi.decode(vm.parseJson(json, ".by"), (bytes32[]));
        require(bx.length == 2 && by.length == 2, "invalid groth16 g2 proof shape");
        solidityProof.b = DivinationPairing.G2Point([uint256(bx[0]), uint256(bx[1])], [uint256(by[0]), uint256(by[1])]);

        solidityProof.c = DivinationPairing.G1Point(
            uint256(abi.decode(vm.parseJson(json, ".cx"), (bytes32))),
            uint256(abi.decode(vm.parseJson(json, ".cy"), (bytes32)))
        );

        proof = abi.encode(solidityProof);

        uint256[DIVINATION_PUBLIC_INPUTS] memory parsedInputs;
        bytes32[] memory parsedRawInputs = abi.decode(vm.parseJson(json, ".publicInputs"), (bytes32[]));
        require(parsedRawInputs.length == DIVINATION_PUBLIC_INPUTS, "invalid public input length");
        for (uint256 i = 0; i < DIVINATION_PUBLIC_INPUTS; i++) {
            parsedInputs[i] = uint256(parsedRawInputs[i]);
        }
        publicInputs = abi.encode(parsedInputs);

        offchainVerified = abi.decode(vm.parseJson(json, ".offchainVerified"), (bool));
    }

    function _generateWinningJudgementFixture()
        internal
        returns (bytes memory proof, bytes memory publicInputs, bool offchainVerified)
    {
        string memory json = _runFixture("winning_judgement_groth16_fixture");

        WinningJudgementGroth16Verifier.Proof memory solidityProof;
        solidityProof.a = WinningJudgementPairing.G1Point(
            uint256(abi.decode(vm.parseJson(json, ".ax"), (bytes32))),
            uint256(abi.decode(vm.parseJson(json, ".ay"), (bytes32)))
        );

        bytes32[] memory bx = abi.decode(vm.parseJson(json, ".bx"), (bytes32[]));
        bytes32[] memory by = abi.decode(vm.parseJson(json, ".by"), (bytes32[]));
        require(bx.length == 2 && by.length == 2, "invalid groth16 g2 proof shape");
        solidityProof.b =
            WinningJudgementPairing.G2Point([uint256(bx[0]), uint256(bx[1])], [uint256(by[0]), uint256(by[1])]);

        solidityProof.c = WinningJudgementPairing.G1Point(
            uint256(abi.decode(vm.parseJson(json, ".cx"), (bytes32))),
            uint256(abi.decode(vm.parseJson(json, ".cy"), (bytes32)))
        );

        proof = abi.encode(solidityProof);

        uint256[WINNING_JUDGEMENT_PUBLIC_INPUTS] memory parsedInputs;
        bytes32[] memory parsedRawInputs = abi.decode(vm.parseJson(json, ".publicInputs"), (bytes32[]));
        require(parsedRawInputs.length == WINNING_JUDGEMENT_PUBLIC_INPUTS, "invalid public input length");
        for (uint256 i = 0; i < WINNING_JUDGEMENT_PUBLIC_INPUTS; i++) {
            parsedInputs[i] = uint256(parsedRawInputs[i]);
        }
        publicInputs = abi.encode(parsedInputs);

        offchainVerified = abi.decode(vm.parseJson(json, ".offchainVerified"), (bool));
    }

    function _generateKeyPublicizeFixture()
        internal
        returns (bytes memory proof, bytes memory publicInputs, bool offchainVerified)
    {
        string memory json = _runFixture("key_publicize_groth16_fixture");

        KeyPublicizeGroth16Verifier.Proof memory solidityProof;
        solidityProof.a = KeyPublicizePairing.G1Point(
            uint256(abi.decode(vm.parseJson(json, ".ax"), (bytes32))),
            uint256(abi.decode(vm.parseJson(json, ".ay"), (bytes32)))
        );

        bytes32[] memory bx = abi.decode(vm.parseJson(json, ".bx"), (bytes32[]));
        bytes32[] memory by = abi.decode(vm.parseJson(json, ".by"), (bytes32[]));
        require(bx.length == 2 && by.length == 2, "invalid groth16 g2 proof shape");
        solidityProof.b = KeyPublicizePairing.G2Point([uint256(bx[0]), uint256(bx[1])], [uint256(by[0]), uint256(by[1])]);

        solidityProof.c = KeyPublicizePairing.G1Point(
            uint256(abi.decode(vm.parseJson(json, ".cx"), (bytes32))),
            uint256(abi.decode(vm.parseJson(json, ".cy"), (bytes32)))
        );

        proof = abi.encode(solidityProof);

        bytes32[] memory parsedRawInputs = abi.decode(vm.parseJson(json, ".publicInputs"), (bytes32[]));
        require(parsedRawInputs.length == 0, "invalid public input length");
        publicInputs = bytes("");

        offchainVerified = abi.decode(vm.parseJson(json, ".offchainVerified"), (bool));
    }

    function _runFixture(string memory binName) internal returns (string memory json) {
        string memory manifestPath = string.concat(vm.projectRoot(), "/../arkworks-solidity-verifier/Cargo.toml");
        string[] memory ffiCmd = new string[](11);
        ffiCmd[0] = "env";
        ffiCmd[1] = "RUSTFLAGS=-Awarnings";
        ffiCmd[2] = "cargo";
        ffiCmd[3] = "run";
        ffiCmd[4] = "--offline";
        ffiCmd[5] = "--locked";
        ffiCmd[6] = "--quiet";
        ffiCmd[7] = "--manifest-path";
        ffiCmd[8] = manifestPath;
        ffiCmd[9] = "--bin";
        ffiCmd[10] = binName;
        return string(vm.ffi(ffiCmd));
    }
}
