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

export interface PedersernCommitment {
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
  playerCommitment: PedersernCommitment[];
  playerNum: number;
}

export interface KeyPublicizePrivateInput {
  id: number;
  pub_key_or_dummy_x: (number[] | null)[];
  pub_key_or_dummy_y: (number[] | null)[];
  is_fortune_teller: (number[] | null)[];
}
export interface KeyPublicizePublicInput {
  pedersenParam: PedersenParam;
}

// TODO: modify.
export interface RoleAssignmentPrivateInput {
  id: number;
  //   isTargetId: string[];
  isTargetId: (number[] | null)[][];
  playerRandomness: (number[] | null)[];
}
export interface RoleAssignmentPublicInput {
  pedersenParam: PedersenParam;
  playerCommitment: PedersernCommitment[];
}

// TODO: modify.
export interface DivinationPrivateInput {
  id: number;
  //   isTargetId: string[];
  isTargetId: (number[] | null)[][];
  playerRandomness: (number[] | null)[];
}
export interface DivinationPublicInput {
  pedersenParam: PedersenParam;
  elgamalParam: any;
  playerCommitment: PedersernCommitment[];
}

// TODO: modify.
export interface WinningJudgementPrivateInput {
  id: number;
  isTargetId: (number[] | null)[][];
  playerRandomness: (number[] | null)[];
}
export interface WinningJudgementPublicInput {
  pedersenParam: PedersenParam;
  playerCommitment: PedersernCommitment[];
}
