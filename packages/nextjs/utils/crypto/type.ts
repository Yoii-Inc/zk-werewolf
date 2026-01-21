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
  publicKey?: string; // Curve25519公開鍵（Base64エンコード）
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

export type Field = bigint[] | null;

export interface PedersenParam {
  randomness_generator: {
    x: Field[];
    y: Field[];
    t: Field[];
    z: Field[];
    _params: null;
  }[];
  generators: {
    x: Field[];
    y: Field[];
    t: Field[];
    z: Field[];
    _params: null;
  }[][];
}

export interface PedersenCommitment {
  x: Field[];
  y: Field[];
  _params: null;
}

export interface ElGamalParam {
  generator: {
    x: Field[];
    y: Field[];
    _params: null;
  };
}

export interface ElGamalPublicKey {
  x: Field[];
  y: Field[];
  _params: null;
}

export type ElGamalSecretKey = Field[];

export interface AnonymousVotingPrivateInput {
  id: number;
  //   isTargetId: string[];
  isTargetId: Field[][];
  playerRandomness: Field[];
}
export interface AnonymousVotingPublicInput {
  pedersenParam: PedersenParam;
  playerCommitment: PedersenCommitment[];
  playerNum: number;
}

export interface KeyPublicizePrivateInput {
  id: number;
  pubKeyOrDummyX: Field[] | null;
  pubKeyOrDummyY: Field[] | null;
  isFortuneTeller: Field[] | null;
}
export interface KeyPublicizePublicInput {
  pedersenParam: PedersenParam;
}

// TODO: modify.
export interface RoleAssignmentPrivateInput {
  id: number;
  shuffleMatrices: any;
  randomness: any;
  playerRandomness: Field[];
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
  isWerewolf: Field[];
  isTarget: Field[][];
  //   randomness: (number[] | null)[];
  randomness: Field[];
}
export interface DivinationPublicInput {
  pedersenParam: PedersenParam;
  elgamalParam: ElGamalParam;
  pubKey: any;
  playerNum: any;
  //   playerCommitment: PedersenCommitment[];
}

export interface WinningJudgementPrivateInput {
  id: number;
  amWerewolf: Field[];
  playerRandomness: Field[];
}
export interface WinningJudgementPublicInput {
  pedersenParam: PedersenParam;
  playerCommitment: PedersenCommitment[];
}

export interface ElGamalDecryptInput {
  elgamalParams: ElGamalParam;
  secretKey: ElGamalSecretKey;
  ciphertext: any;
}

export interface ElGamalDecryptOutput {
  plaintext: any;
}

export interface ElGamalKeygenInput {
  elgamalParams: ElGamalParam;
}

export interface ElGamalKeygenOutput {
  publicKey: ElGamalPublicKey;
  secretKey: ElGamalSecretKey;
}

export interface ElGamalEncryptInput {
  elgamalParams: ElGamalParam;
  publicKey: ElGamalPublicKey;
  message: any;
  randomness: Field[];
}
