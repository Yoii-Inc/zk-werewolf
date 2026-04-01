import { CryptoHelper } from "./helpers/crypto";
import { GameSetupHelper, checkWebSocketConnections, createTestSetup } from "./setup";
import { GameInfo } from "~~/types/game";
import { getPrivateGameInfo } from "~~/utils/privateGameInfoUtils";

type DivinationStrategy = "next-player" | "last-player";
type VotingStrategy = "ring" | "focus-player-2" | "split-vote";

interface FullScenario {
  id: string;
  numPlayers: number;
  werewolfCount: number;
  divinationStrategy: DivinationStrategy;
  votingStrategy: VotingStrategy;
  simulateNightAttackBeforeDivination?: boolean;
}

const fullScenarios: FullScenario[] = [
  {
    id: "n4-w1-ring",
    numPlayers: 4,
    werewolfCount: 1,
    divinationStrategy: "next-player",
    votingStrategy: "ring",
  },
  {
    id: "n5-w1-ring",
    numPlayers: 5,
    werewolfCount: 1,
    divinationStrategy: "next-player",
    votingStrategy: "ring",
  },
  {
    id: "n5-w2-focus",
    numPlayers: 5,
    werewolfCount: 2,
    divinationStrategy: "last-player",
    votingStrategy: "focus-player-2",
  },
  {
    id: "n5-w2-split-vote",
    numPlayers: 5,
    werewolfCount: 2,
    divinationStrategy: "last-player",
    votingStrategy: "split-vote",
  },
  {
    id: "n5-w2-attack-divination",
    numPlayers: 5,
    werewolfCount: 2,
    divinationStrategy: "last-player",
    votingStrategy: "focus-player-2",
    simulateNightAttackBeforeDivination: true,
  },
];

function getPlayerIdByIndex(gameState: GameInfo, index: number): string {
  const normalized = ((index % gameState.players.length) + gameState.players.length) % gameState.players.length;
  return gameState.players[normalized]?.id ?? gameState.players[0]?.id ?? "1";
}

function buildDivinationTargets(gameState: GameInfo, strategy: DivinationStrategy): string[] {
  if (strategy === "last-player") {
    const targetId = getPlayerIdByIndex(gameState, gameState.players.length - 1);
    return gameState.players.map(() => targetId);
  }

  return gameState.players.map((_, i) => getPlayerIdByIndex(gameState, i + 1));
}

function buildVotingTargets(gameState: GameInfo, strategy: VotingStrategy): string[] {
  if (strategy === "focus-player-2") {
    const targetId = getPlayerIdByIndex(gameState, 1);
    return gameState.players.map(() => targetId);
  }

  if (strategy === "split-vote") {
    const splitIndexes =
      gameState.players.length >= 5 ? [1, 1, 2, 2, 3] : gameState.players.map((_, i) => (i < 2 ? 1 : 2));
    return gameState.players.map((_, i) => getPlayerIdByIndex(gameState, splitIndexes[i] ?? 1));
  }

  return gameState.players.map((_, i) => getPlayerIdByIndex(gameState, i + 1));
}

async function runFullScenarioFlow(scenario: FullScenario): Promise<void> {
  const { roomId, players } = {
    roomId: global.testRoomId,
    players: global.testPlayers,
  };

  expect(players.length).toBe(scenario.numPlayers);
  await checkWebSocketConnections();

  let gameState: GameInfo = await global.apiClient.getGameState(roomId);

  await GameSetupHelper.submitPlayerCommitments(roomId, players, gameState);
  await new Promise(resolve => setTimeout(resolve, 3000));
  await GameSetupHelper.submitRoleAssignmentRequests(roomId, players, gameState);
  const deliveries = global.roleAssignmentDeliveries ?? [];
  expect(deliveries.length).toBeGreaterThan(0);
  deliveries.forEach(delivery => {
    expect(delivery.receiverPlayerId).toBe(delivery.targetPlayerId);
  });

  await new Promise(resolve => setTimeout(resolve, 1000));
  const privateInfos = players
    .map(player => getPrivateGameInfo(roomId, player.id))
    .filter((info): info is NonNullable<typeof info> => Boolean(info));
  const werewolves = privateInfos.filter(info => info.playerRole === "Werewolf");
  const seerPlayerId = privateInfos.find(info => info.playerRole === "Seer")?.playerId;

  if (scenario.werewolfCount >= 2) {
    const werewolfIdSet = new Set(werewolves.map(info => info.playerId));

    expect(werewolves.length).toBe(scenario.werewolfCount);

    werewolves.forEach(werewolfInfo => {
      const teammateIds = werewolfInfo.werewolfTeammateIds ?? [];
      expect(teammateIds.length).toBe(scenario.werewolfCount - 1);
      expect(teammateIds).not.toContain(werewolfInfo.playerId);
      teammateIds.forEach(teammateId => {
        expect(werewolfIdSet.has(teammateId)).toBe(true);
      });
    });

    privateInfos
      .filter(info => info.playerRole !== "Werewolf")
      .forEach(info => {
        expect(info.werewolfTeammateIds ?? []).toHaveLength(0);
      });
  }

  gameState = await global.apiClient.getGameState(roomId);
  await GameSetupHelper.submitKeyPublicizeRequests(roomId, players, gameState);

  let nightAttackTargetId: string | null = null;
  if (scenario.simulateNightAttackBeforeDivination) {
    gameState = await GameSetupHelper.ensureGamePhase(roomId, "Night");
    expect(seerPlayerId).toBeDefined();

    const werewolfIdSet = new Set(werewolves.map(info => info.playerId));
    const attackerPlayerId = werewolves[0]?.playerId ?? players[0].id;
    const attacker = players.find(player => player.id === attackerPlayerId) ?? players[0];
    const targetPlayer =
      gameState.players.find(
        player => !player.is_dead && player.id !== seerPlayerId && !werewolfIdSet.has(player.id),
      ) ??
      gameState.players.find(player => !player.is_dead && player.id !== attacker.id) ??
      gameState.players[0];

    nightAttackTargetId = targetPlayer.id;
    await GameSetupHelper.submitNightAttack(roomId, attacker, nightAttackTargetId);

    const stateAfterAttack = await global.apiClient.getGameState(roomId);
    const attackedPlayerAfterAttack = stateAfterAttack.players.find(player => player.id === nightAttackTargetId);
    expect(attackedPlayerAfterAttack?.is_dead).toBe(false);
  }

  gameState = await GameSetupHelper.ensureGamePhase(roomId, "DivinationProcessing");
  if (nightAttackTargetId) {
    const attackedPlayer = gameState.players.find(player => player.id === nightAttackTargetId);
    expect(attackedPlayer?.is_dead).toBe(false);
  }
  if (!gameState.crypto_parameters?.fortune_teller_public_key) {
    const cryptoParams = await CryptoHelper.loadParams();
    gameState.crypto_parameters = {
      ...(gameState.crypto_parameters ?? {}),
      fortune_teller_public_key: cryptoParams.fortune_teller_public_key,
    } as any;
  }

  const divinationTargets = nightAttackTargetId
    ? gameState.players.map(() => nightAttackTargetId as string)
    : buildDivinationTargets(gameState, scenario.divinationStrategy);
  const isDummyFlags = players.map(player => (seerPlayerId ? player.id !== seerPlayerId : player.id !== players[0].id));
  await GameSetupHelper.submitDivinationRequests(roomId, players, gameState, divinationTargets, isDummyFlags);

  if (nightAttackTargetId) {
    gameState = await GameSetupHelper.ensureGamePhase(roomId, "Discussion");
    const attackedPlayer = gameState.players.find(player => player.id === nightAttackTargetId);
    expect(attackedPlayer?.is_dead).toBe(true);
  }

  gameState = await GameSetupHelper.ensureGamePhase(roomId, "Voting");
  const votingTargets = buildVotingTargets(gameState, scenario.votingStrategy);
  await GameSetupHelper.submitVotingRequests(roomId, players, gameState, votingTargets);

  gameState = await GameSetupHelper.ensureGamePhase(roomId, "Result");
  await GameSetupHelper.submitWinningJudgementRequests(roomId, players, gameState);

  const finalState = await global.apiClient.getGameState(roomId);
  expect(finalState.players.length).toBe(scenario.numPlayers);
}

const describeFull = process.env.E2E_SCENARIO_SET === "full" ? describe : describe.skip;

describeFull("ZK Werewolf Integration E2E Tests (Full Scenarios)", () => {
  describe.each(fullScenarios)("$id", scenario => {
    const scenarioSetup = createTestSetup({
      numPlayers: scenario.numPlayers,
      werewolfCount: scenario.werewolfCount,
      roomNamePrefix: `E2E Full ${scenario.id}`,
    });

    beforeAll(scenarioSetup.beforeAll);
    beforeEach(scenarioSetup.beforeEach);
    afterAll(scenarioSetup.afterAll);

    test(`full flow works (${scenario.numPlayers} players, ${scenario.werewolfCount} werewolves)`, async () => {
      await runFullScenarioFlow(scenario);
    }, 600000);
  });
});
