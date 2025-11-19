export interface NodeKey {
  nodeId: string;
  publicKey: string;
}

export interface SecretSharingScheme {
  totalShares: number;
  modulus: number;
}

export type AnonymousVotingInput = {
  privateInput: AnonymousVotingPrivateInput;
  publicInput: AnonymousVotingPublicInput;
  nodeKeys: NodeKey[];
  scheme: SecretSharingScheme;
};

export type AnonymousVotingOutput = string;

export type KeyPublicizeInput = {
  privateInput: KeyPublicizePrivateInput;
  publicInput: KeyPublicizePublicInput;
  nodeKeys: NodeKey[];
  scheme: SecretSharingScheme;
};

export type KeyPublicizeOutput = string;

export type RoleAssignmentInput = {
  privateInput: RoleAssignmentPrivateInput;
  publicInput: RoleAssignmentPublicInput;
  nodeKeys: NodeKey[];
  scheme: SecretSharingScheme;
};

export type RoleAssignmentOutput = string;

export type DivinationInput = {
  privateInput: DivinationPrivateInput;
  publicInput: DivinationPublicInput;
  nodeKeys: NodeKey[];
  scheme: SecretSharingScheme;
};

export type DivinationOutput = string;

export type WinningJudgementInput = {
  privateInput: WinningJudgementPrivateInput;
  publicInput: WinningJudgementPublicInput;
  nodeKeys: NodeKey[];
  scheme: SecretSharingScheme;
};

export type WinningJudgementOutput = string;

////////////////////

export interface PedersenParam {
  randomness_generator: {
    x: (bigint[] | null)[];
    y: (bigint[] | null)[];
    t: (bigint[] | null)[];
    z: (bigint[] | null)[];
    _params: null;
  }[];
  generators: {
    x: (bigint[] | null)[];
    y: (bigint[] | null)[];
    t: (bigint[] | null)[];
    z: (bigint[] | null)[];
    _params: null;
  }[][];
}

export interface PedersenCommitment {
  x: (number[] | null)[];
  y: (number[] | null)[];
  _params: null;
}

export interface AnonymousVotingPrivateInput {
  id: number;
  //   isTargetId: string[];
  isTargetId: (number[] | null)[][];
  playerRandomness: (number[] | null)[];
}
export interface AnonymousVotingPublicInput {
  pedersenParam: PedersenParam;
  playerCommitment: PedersenCommitment[];
  playerNum: number;
}

export interface KeyPublicizePrivateInput {
  id: number;
  pub_key_or_dummy_x: number[] | null;
  pub_key_or_dummy_y: number[] | null;
  is_fortune_teller: number[] | null;
}
export interface KeyPublicizePublicInput {
  pedersenParam: PedersenParam;
}

// TODO: modify.
export interface RoleAssignmentPrivateInput {
  id: number;
  shuffleMatrices: any;
  randomness: any;
  playerRandomness: number[] | null;
}
export interface RoleAssignmentPublicInput {
  // parameter
  numPlayers: number;
  maxGroupSize: number;
  pedersenParam: PedersenParam;
  groupingParameter: GroupingParameter;

  // instance
  tauMatrix: any;
  roleCommitment: PedersenCommitment[];
  playerCommitment: PedersenCommitment[];
}

type GroupingParameter = {
  Villager: [number, boolean];
  FortuneTeller: [number, boolean];
  Werewolf: [number, boolean];
};

// TODO: modify.
export interface DivinationPrivateInput {
  id: number;
  isWerewolf: (bigint[] | null)[];
  isTarget: (bigint[] | null)[][];
  //   randomness: (number[] | null)[];
  randomness: any;
}
export interface DivinationPublicInput {
  pedersenParam: PedersenParam;
  elgamalParam: any;
  pubKey: any;
  playerNum: any;
  //   playerCommitment: PedersenCommitment[];
}

export interface WinningJudgementPrivateInput {
  id: number;
  amWerewolf: (number[] | null)[];
  playerRandomness: (number[] | null)[];
}
export interface WinningJudgementPublicInput {
  pedersenParam: PedersenParam;
  playerCommitment: PedersenCommitment[];
}
