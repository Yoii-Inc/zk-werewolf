# ブロックチェーン統合 実装計画

## 概要

ZK Werewolfゲームにおけるブロックチェーン統合により、ゲームの公平性を検証可能にし、試合結果をオンチェーンで記録・確認できるようにする。

## Scaffold-ETH 前提での実装方針

- コントラクトデプロイは `packages/foundry` の `yarn deploy` を利用する（Scaffold-ETH の標準フロー）。
- `packages/nextjs/contracts/deployedContracts.ts` は `yarn deploy` 実行時に自動生成・更新されるため手編集しない。
- フロントエンドは `wagmi` の生フックより、既存の `hooks/scaffold-eth/*`（`useScaffoldReadContract` / `useScaffoldWriteContract` / `useScaffoldEventHistory`）を優先利用する。
- ネットワークは `packages/nextjs/scaffold.config.ts` の `targetNetworks` を唯一の参照元にし、`31337` のハードコードを避ける。

## 現在の実装状況

### 実装済み
- ✅ バックエンドのゲーム状態管理 (`Game`, `GamePhase`, `GameResult`)
- ✅ コミットメント管理システム (`submit_commitment`)
- ✅ ZK証明リクエスト処理 (`proof_handler`, `batch_proof_handling`)
- ✅ 計算結果の保存 (`ComputationResults`)
- ✅ ZK-MPCノードでの証明生成 (`ProofManager`)
- ✅ フロントエンドのWeb3統合 (wagmi, RainbowKit)
- ✅ 基本的なスマートコントラクト (`YourContract`)

### 未実装
- ❌ ゲーム専用スマートコントラクト
- ❌ チェーンへのゲーム状態/コミットメントの記録
- ❌ オンチェーン証明検証
- ❌ 試合結果のオンチェーン記録
- ❌ 報酬システム
- ❌ 試合履歴確認UI

---

## 実装計画

### Phase 1: スマートコントラクト基盤構築

#### 1.1 WerewolfGameコントラクトの実装

**ファイル**: `packages/foundry/contracts/WerewolfGame.sol`

**機能**:
- ゲーム状態のハッシュ記録
- プレイヤーコミットメントの記録
- ゲーム登録・開始・終了の管理
- アクセス制御（ゲームマスター権限）

**データ構造**:
```solidity
struct GameState {
    bytes32 stateHash;           // ゲーム状態のMerkleルートまたはハッシュ
    address[] players;           // プレイヤーアドレス
    uint256 startTime;           // 開始時刻
    uint256 endTime;             // 終了時刻
    GameStatus status;           // ゲーム状態（Waiting, InProgress, Finished）
    GameResult result;           // 結果（InProgress, VillagerWin, WerewolfWin）
}

struct PlayerCommitment {
    bytes32 commitment;          // Pedersenコミットメント
    uint256 timestamp;           // タイムスタンプ
}

mapping(bytes32 => GameState) public games;           // gameId => GameState
mapping(bytes32 => mapping(address => PlayerCommitment)) public commitments;
```

**主要関数**:
- `createGame(bytes32 gameId, address[] players)`: ゲーム作成
- `submitCommitment(bytes32 gameId, bytes32 commitment)`: コミットメント提出
- `updateGameState(bytes32 gameId, bytes32 stateHash)`: ゲーム状態更新
- `finalizeGame(bytes32 gameId, GameResult result)`: ゲーム終了
- `verifyGameState(bytes32 gameId, bytes32 expectedHash)`: 状態検証

**イベント**:
```solidity
event GameCreated(bytes32 indexed gameId, address[] players, uint256 timestamp);
event CommitmentSubmitted(bytes32 indexed gameId, address indexed player, bytes32 commitment);
event GameStateUpdated(bytes32 indexed gameId, bytes32 stateHash, uint256 timestamp);
event GameFinalized(bytes32 indexed gameId, GameResult result, uint256 timestamp);
```

#### 1.2 ZK証明検証コントラクトの実装

**ファイル**: `packages/foundry/contracts/WerewolfProofVerifier.sol`

**機能**:
- ZK証明の検証（Marlin/Groth16）
- 証明タイプ別の検証ロジック
- 検証結果の記録

**データ構造**:
```solidity
enum ProofType {
    RoleAssignment,
    Divination,
    AnonymousVoting,
    WinningJudgement,
    KeyPublicize
}

struct ProofRecord {
    ProofType proofType;
    bytes32 gameId;
    bytes32 proofHash;
    bool verified;
    uint256 timestamp;
}

mapping(bytes32 => ProofRecord) public proofs;  // proofId => ProofRecord
```

**主要関数**:
- `verifyProof(bytes32 proofId, bytes calldata proof, bytes calldata publicInputs)`: 証明検証
- `getProofRecord(bytes32 proofId)`: 証明記録取得
- `isProofVerified(bytes32 gameId, ProofType proofType)`: 証明済み確認

**イベント**:
```solidity
event ProofVerified(bytes32 indexed proofId, bytes32 indexed gameId, ProofType proofType, uint256 timestamp);
event ProofFailed(bytes32 indexed proofId, bytes32 indexed gameId, ProofType proofType, string reason);
```

#### 1.3 報酬・インセンティブコントラクトの実装

**ファイル**: `packages/foundry/contracts/WerewolfRewards.sol`

**機能**:
- 勝利報酬の管理
- 賞金プールの管理
- 報酬分配ロジック

**データ構造**:
```solidity
struct RewardPool {
    uint256 totalAmount;         // 総賞金額
    uint256 entryFee;           // 参加費
    uint256 winnerShare;        // 勝者への分配率（basis points）
}

mapping(bytes32 => RewardPool) public gamePools;
mapping(bytes32 => mapping(address => bool)) public claimed;
```

**主要関数**:
- `depositReward(bytes32 gameId)`: 報酬預託（payable）
- `claimReward(bytes32 gameId)`: 報酬請求
- `distributeRewards(bytes32 gameId, address[] winners)`: 報酬分配（オーナー/ゲームコントラクトのみ）

**イベント**:
```solidity
event RewardDeposited(bytes32 indexed gameId, address indexed player, uint256 amount);
event RewardClaimed(bytes32 indexed gameId, address indexed winner, uint256 amount);
event RewardsDistributed(bytes32 indexed gameId, uint256 totalAmount, uint256 winnerCount);
```

#### 1.4 デプロイスクリプトの作成

**ファイル**: `packages/foundry/script/DeployWerewolf.s.sol`

```solidity
contract DeployWerewolf is Script {
    function run() external {
        vm.startBroadcast();
        
        WerewolfGame game = new WerewolfGame();
        WerewolfProofVerifier verifier = new WerewolfProofVerifier();
        WerewolfRewards rewards = new WerewolfRewards(address(game));
        
        // 権限設定
        game.setVerifier(address(verifier));
        game.setRewardsContract(address(rewards));
        
        vm.stopBroadcast();
    }
}
```

---

### Phase 2: バックエンド統合

#### 2.1 ブロックチェーン接続モジュールの実装

**ファイル**: `packages/server/src/blockchain/mod.rs`

**機能**:
- Ethereumノードへの接続（alloy使用）
- コントラクト呼び出しラッパー
- トランザクション送信・確認
- ローカルAnvil/L2テストネットの切り替え

**実装内容**:
```rust
pub struct BlockchainClient {
    rpc_url: String,
    chain_id: u64,
    signer: PrivateKeySigner,
    game_contract: WerewolfGameClient,                 // alloy-sol-types で生成した型付きクライアント
    verifier_contract: WerewolfProofVerifierClient,    // 同上
    rewards_contract: WerewolfRewardsClient,           // 同上
}

impl BlockchainClient {
    pub async fn new(rpc_url: &str, private_key: &str, contract_addresses: ContractAddresses) -> Result<Self>;
    
    // ゲーム管理
    pub async fn create_game(&self, game_id: &str, players: Vec<Address>) -> Result<B256>;
    pub async fn submit_commitment(&self, game_id: &str, player: Address, commitment: [u8; 32]) -> Result<B256>;
    pub async fn update_game_state(&self, game_id: &str, state_hash: [u8; 32]) -> Result<B256>;
    pub async fn finalize_game(&self, game_id: &str, result: GameResult) -> Result<B256>;
    
    // 証明検証
    pub async fn verify_proof(&self, proof_id: &str, proof_data: &[u8], public_inputs: &[u8]) -> Result<bool>;
    
    // 報酬管理
    pub async fn distribute_rewards(&self, game_id: &str, winners: Vec<Address>) -> Result<B256>;
    
    // 状態取得
    pub async fn get_game_state(&self, game_id: &str) -> Result<OnChainGameState>;
    pub async fn get_proof_record(&self, proof_id: &str) -> Result<ProofRecord>;
}
```

**補足（ローカル/L2切り替え）**:
- `chain_id` と `rpc_url` を環境変数で切り替える（例: `31337` → `84532` / `421614` / `11155420` など）。
- L2テストネットでは EIP-1559 の `max_fee_per_gas` / `max_priority_fee_per_gas` を明示設定できるようにする。
- トランザクション送信後は `receipt.status` と `block_number` を確認し、再試行戦略（RPC一時エラー）を実装する。

#### 2.2 ゲーム状態のハッシュ化

**ファイル**: `packages/server/src/blockchain/state_hash.rs`

**機能**:
- ゲーム状態の決定論的ハッシュ計算
- Merkleツリー構築（プレイヤー状態、アクション履歴等）

**実装内容**:
```rust
pub fn compute_game_state_hash(game: &Game) -> [u8; 32] {
    let mut hasher = Keccak256::new();
    
    // ゲームID
    hasher.update(game.room_id.as_bytes());
    
    // フェーズと日数
    hasher.update(&[game.phase as u8]);
    hasher.update(&game.day_count.to_le_bytes());
    
    // プレイヤー状態（ソート済み）
    let mut player_hashes: Vec<[u8; 32]> = game.players.iter()
        .map(|p| compute_player_hash(p))
        .collect();
    player_hashes.sort();
    for hash in player_hashes {
        hasher.update(&hash);
    }
    
    // アクション履歴
    hasher.update(&serialize_actions(&game.night_actions));
    
    hasher.finalize().into()
}

fn compute_player_hash(player: &Player) -> [u8; 32] {
    // プレイヤーID、生存状態、その他の状態をハッシュ化
}
```

#### 2.3 ゲームライフサイクルへの統合

**ファイル**: `packages/server/src/services/game_service.rs` (既存ファイルの拡張)

**追加機能**:
```rust
// ゲーム開始時
pub async fn start_game(state: AppState, room_id: &str) -> Result<String, String> {
    // 既存のロジック...
    
    // ブロックチェーンにゲームを登録
    let player_addresses = extract_player_addresses(&game)?;
    let game_id = compute_game_id(room_id);
    
    state.blockchain_client
        .create_game(&game_id, player_addresses)
        .await
        .map_err(|e| format!("Failed to create game on-chain: {}", e))?;
    
    // 初期状態ハッシュを記録
    let state_hash = compute_game_state_hash(&game);
    state.blockchain_client
        .update_game_state(&game_id, state_hash)
        .await
        .map_err(|e| format!("Failed to update game state on-chain: {}", e))?;
    
    // 既存のロジック...
}

// フェーズ遷移時
pub async fn advance_game_phase(state: AppState, room_id: &str) -> Result<String, String> {
    // 既存のロジック...
    
    // 状態変更後、ハッシュを更新
    let state_hash = compute_game_state_hash(&game);
    let game_id = compute_game_id(room_id);
    
    state.blockchain_client
        .update_game_state(&game_id, state_hash)
        .await
        .map_err(|e| format!("Failed to update game state on-chain: {}", e))?;
    
    // 既存のロジック...
}

// ゲーム終了時
pub async fn end_game(state: AppState, room_id: String) -> Result<String, String> {
    // 既存のロジック...
    
    let game_id = compute_game_id(&room_id);
    
    // 最終状態ハッシュ
    let state_hash = compute_game_state_hash(&game);
    state.blockchain_client
        .update_game_state(&game_id, state_hash)
        .await
        .map_err(|e| format!("Failed to update final game state: {}", e))?;
    
    // ゲームを終了
    state.blockchain_client
        .finalize_game(&game_id, game.result.clone().into())
        .await
        .map_err(|e| format!("Failed to finalize game on-chain: {}", e))?;
    
    // 勝者への報酬分配
    if game.result != GameResult::InProgress {
        let winners = get_winning_players(&game)?;
        state.blockchain_client
            .distribute_rewards(&game_id, winners)
            .await
            .map_err(|e| format!("Failed to distribute rewards: {}", e))?;
    }
    
    // 既存のロジック...
}
```

#### 2.4 コミットメント管理の統合

**ファイル**: `packages/server/src/routes/game.rs` (既存ファイルの拡張)

```rust
async fn submit_commitment(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
    Json(commitment_req): Json<CommitmentRequest>,
) -> impl IntoResponse {
    // 既存のロジック...
    
    // ブロックチェーンにコミットメントを記録
    let game_id = compute_game_id(&room_id);
    let player_address = get_player_address(&commitment_req.player_id)?;
    let commitment_hash = serialize_commitment(&commitment_obj);
    
    state.blockchain_client
        .submit_commitment(&game_id, player_address, commitment_hash)
        .await
        .map_err(|e| {
            tracing::error!("Failed to submit commitment on-chain: {}", e);
            // エラーログのみ、オフチェーンは継続
        });
    
    // 既存のロジック...
}
```

#### 2.5 証明検証の統合

**ファイル**: `packages/server/src/services/zk_proof.rs` (既存ファイルの拡張)

```rust
pub async fn batch_proof_handling(
    state: AppState,
    room_id: &str,
    request: &ClientRequestType,
) -> Result<String, String> {
    // 既存のロジック...
    
    // 証明生成完了後、オンチェーン検証を実施
    let proof_output = check_proof_status(&proof_id).await?;
    
    if let Some(output) = proof_output.output {
        if let Some(proof_data) = output.value {
            let game_id = compute_game_id(room_id);
            let proof_id_hash = compute_proof_id(&batch_id);
            
            // 公開入力の構築
            let public_inputs = extract_public_inputs(&request, &game)?;
            
            // オンチェーン検証
            let verified = state.blockchain_client
                .verify_proof(&proof_id_hash, &proof_data, &public_inputs)
                .await
                .map_err(|e| format!("On-chain verification failed: {}", e))?;
            
            if !verified {
                return Err("Proof verification failed on-chain".to_string());
            }
            
            tracing::info!("Proof {} verified on-chain successfully", proof_id_hash);
        }
    }
    
    // 既存のロジック...
}
```

#### 2.6 環境変数・設定の追加

**ファイル**: `packages/server/.env.example`

```env
# Blockchain Configuration
BLOCKCHAIN_ENABLED=true
ETHEREUM_RPC_URL=http://localhost:8545
ETHEREUM_CHAIN_ID=31337
DEPLOYER_PRIVATE_KEY=0x...

# Contract Addresses
WEREWOLF_GAME_CONTRACT=0x...
WEREWOLF_VERIFIER_CONTRACT=0x...
WEREWOLF_REWARDS_CONTRACT=0x...

# Gas Configuration
GAS_LIMIT=3000000
GAS_PRICE_GWEI=20
```

**ファイル**: `packages/server/src/utils/config.rs` (既存ファイルの拡張)

```rust
pub struct BlockchainConfig {
    pub enabled: bool,
    pub rpc_url: String,
    pub chain_id: u64,
    pub private_key: String,
    pub game_contract: String,
    pub verifier_contract: String,
    pub rewards_contract: String,
}

impl BlockchainConfig {
    pub fn from_env() -> Self {
        // 環境変数から読み込み
    }
}
```

---

### Phase 3: フロントエンド統合

#### 3.1 コントラクトインターフェースの生成

**手順**:
1. Foundryでコントラクトをコンパイル
2. Scaffold-ETH の deploy スクリプト経由でデプロイし、ABI/アドレスを自動反映

```bash
cd packages/foundry
yarn deploy --file DeployWerewolf.s.sol --network localhost
```

**結果**:
- `packages/nextjs/contracts/deployedContracts.ts` が自動更新される
- デプロイ履歴は `packages/foundry/broadcast/*` に保存される

#### 3.2 ゲーム状態確認フック

**ファイル**: `packages/nextjs/hooks/useGameContract.ts`

```typescript
import { useScaffoldReadContract } from "~~/hooks/scaffold-eth";

export const useGameContract = (gameId: string) => {
  const { data: gameState, refetch } = useScaffoldReadContract({
    contractName: "WerewolfGame",
    functionName: "games",
    args: [gameId],
  });

  const { data: commitments } = useScaffoldReadContract({
    contractName: "WerewolfGame",
    functionName: "getCommitments",
    args: [gameId],
  });

  return {
    gameState,
    commitments,
    refetch,
  };
};
```

#### 3.3 コミットメント提出フック

**ファイル**: `packages/nextjs/hooks/useCommitmentSubmission.ts`

```typescript
import { useScaffoldWriteContract } from "~~/hooks/scaffold-eth";

export const useCommitmentSubmission = () => {
  const { writeContractAsync } = useScaffoldWriteContract({ contractName: "WerewolfGame" });
  const { address } = useAccount();

  const submitCommitment = async (gameId: string, commitment: string) => {
    if (!address) throw new Error('No wallet connected');

    // オフチェーンAPIにも送信
    const response = await fetch(`/api/game/${gameId}/commitment`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ player_id: address, commitment }),
    });

    if (!response.ok) {
      throw new Error('Failed to submit commitment to server');
    }

    // オンチェーンにも記録
    const tx = await writeContractAsync({
      functionName: "submitCommitment",
      args: [gameId, commitment],
    });

    return tx;
  };

  return { submitCommitment };
};
```

#### 3.4 試合履歴表示コンポーネント

**ファイル**: `packages/nextjs/app/record/page.tsx` (既存ファイルの拡張)

```typescript
export default function GameRecordsPage() {
  const [gameIds, setGameIds] = useState<string[]>([]);
  
  // 過去のゲームIDを取得（イベントログから）
  const { data: gameCreatedEvents } = useScaffoldEventHistory({
    contractName: "WerewolfGame",
    eventName: "GameCreated",
    fromBlock: 0n,
  });

  useEffect(() => {
    if (gameCreatedEvents) {
      const ids = gameCreatedEvents.map(event => event.args.gameId);
      setGameIds(ids);
    }
  }, [gameCreatedEvents]);

  return (
    <div className="container mx-auto p-4">
      <h1 className="text-2xl font-bold mb-4">試合記録</h1>
      <div className="grid gap-4">
        {gameIds.map(gameId => (
          <GameRecordCard key={gameId} gameId={gameId} />
        ))}
      </div>
    </div>
  );
}
```

**ファイル**: `packages/nextjs/components/game/GameRecordCard.tsx`

```typescript
export const GameRecordCard = ({ gameId }: { gameId: string }) => {
  const { data: gameState } = useScaffoldReadContract({
    contractName: "WerewolfGame",
    functionName: "games",
    args: [gameId],
  });

  if (!gameState) return <div>Loading...</div>;

  return (
    <div className="card bg-base-200 shadow-xl">
      <div className="card-body">
        <h2 className="card-title">Game ID: {gameId.slice(0, 10)}...</h2>
        <div className="stats stats-vertical lg:stats-horizontal">
          <div className="stat">
            <div className="stat-title">状態</div>
            <div className="stat-value text-sm">{gameState.status}</div>
          </div>
          <div className="stat">
            <div className="stat-title">結果</div>
            <div className="stat-value text-sm">{gameState.result}</div>
          </div>
          <div className="stat">
            <div className="stat-title">プレイヤー数</div>
            <div className="stat-value text-sm">{gameState.players.length}</div>
          </div>
        </div>
        <div className="card-actions justify-end">
          <Link href={`/record/${gameId}`} className="btn btn-primary btn-sm">
            詳細を見る
          </Link>
        </div>
      </div>
    </div>
  );
};
```

#### 3.5 詳細画面の実装

**ファイル**: `packages/nextjs/app/record/[gameId]/page.tsx`

```typescript
export default function GameDetailPage({ params }: { params: { gameId: string } }) {
  const { gameState } = useGameContract(params.gameId);
  
  const { data: stateUpdates } = useScaffoldEventHistory({
    contractName: "WerewolfGame",
    eventName: "GameStateUpdated",
    filters: { gameId: params.gameId },
    fromBlock: 0n,
  });

  const { data: proofRecords } = useScaffoldEventHistory({
    contractName: "WerewolfProofVerifier",
    eventName: "ProofVerified",
    filters: { gameId: params.gameId },
    fromBlock: 0n,
  });

  return (
    <div className="container mx-auto p-4">
      <h1 className="text-3xl font-bold mb-6">試合詳細</h1>
      
      {/* ゲーム基本情報 */}
      <GameInfoCard gameState={gameState} />
      
      {/* 状態遷移履歴 */}
      <StateHistoryTimeline updates={stateUpdates} />
      
      {/* 証明検証履歴 */}
      <ProofVerificationHistory proofs={proofRecords} />
      
      {/* プレイヤー一覧 */}
      <PlayersList players={gameState?.players} />
      
      {/* Block Explorerへのリンク */}
      <ExplorerLinks gameId={params.gameId} />
    </div>
  );
}
```

#### 3.6 ブロックエクスプローラーリンク

**ファイル**: `packages/nextjs/components/game/ExplorerLinks.tsx`

```typescript
export const ExplorerLinks = ({ gameId }: { gameId: string }) => {
  const chainId = useChainId();
  const { data: gameContract } = useDeployedContractInfo({ contractName: "WerewolfGame" });
  const explorerUrl = chainId === 31337 
    ? '/blockexplorer' // Scaffold-ETH内蔵のローカルBlock Explorer
    : `https://sepolia.etherscan.io`; // テストネット

  return (
    <div className="card bg-base-200 shadow-xl mt-4">
      <div className="card-body">
        <h3 className="card-title">Block Explorer</h3>
        <div className="flex gap-2">
          <a 
            href={`${explorerUrl}/tx/${/* Transaction Hash */}`}
            target="_blank"
            rel="noopener noreferrer"
            className="btn btn-sm btn-outline"
          >
            トランザクションを見る
          </a>
          <a 
            href={`${explorerUrl}/address/${gameContract?.address}`}
            target="_blank"
            rel="noopener noreferrer"
            className="btn btn-sm btn-outline"
          >
            コントラクトを見る
          </a>
        </div>
      </div>
    </div>
  );
};
```

---

### Phase 4: テストとデプロイ

#### 4.1 スマートコントラクトのテスト

**ファイル**: `packages/foundry/test/WerewolfGame.t.sol`

```solidity
contract WerewolfGameTest is Test {
    WerewolfGame game;
    WerewolfProofVerifier verifier;
    WerewolfRewards rewards;
    
    address player1 = address(0x1);
    address player2 = address(0x2);
    
    function setUp() public {
        game = new WerewolfGame();
        verifier = new WerewolfProofVerifier();
        rewards = new WerewolfRewards(address(game));
        
        game.setVerifier(address(verifier));
        game.setRewardsContract(address(rewards));
    }
    
    function testCreateGame() public {
        bytes32 gameId = keccak256("test-game-1");
        address[] memory players = new address[](2);
        players[0] = player1;
        players[1] = player2;
        
        vm.expectEmit(true, false, false, true);
        emit GameCreated(gameId, players, block.timestamp);
        
        game.createGame(gameId, players);
        
        (bytes32 stateHash, , , , , ) = game.games(gameId);
        assertTrue(stateHash == bytes32(0)); // 初期状態
    }
    
    function testSubmitCommitment() public {
        // テストロジック
    }
    
    function testUpdateGameState() public {
        // テストロジック
    }
    
    function testFinalizeGame() public {
        // テストロジック
    }
}
```

**実行**:
```bash
cd packages/foundry
forge test -vvv
```

#### 4.2 統合テストの実装

**ファイル**: `packages/server/tests/blockchain_integration_test.rs`

```rust
#[tokio::test]
async fn test_full_game_lifecycle_with_blockchain() {
    // 1. ゲーム作成
    // 2. プレイヤー参加
    // 3. コミットメント提出（オンチェーン確認）
    // 4. 役職配布証明（オンチェーン検証）
    // 5. ゲーム進行
    // 6. 勝利判定証明（オンチェーン検証）
    // 7. ゲーム終了（状態確認）
    // 8. 報酬分配（残高確認）
}
```

#### 4.3 ローカル環境でのテスト

**手順**:
1. Anvilを起動
```bash
yarn chain
```

2. コントラクトをデプロイ
```bash
yarn deploy --file DeployWerewolf.s.sol --network localhost
```

3. バックエンドを起動（環境変数設定済み）
```bash
cd packages/server
BLOCKCHAIN_ENABLED=true cargo run
```

4. フロントエンドを起動
```bash
# 別ターミナル（リポジトリルート）
yarn start
```

5. 統合テストを実行
```bash
./integration_test.zsh --with-blockchain
```

#### 4.4 テストネットデプロイ（+α）

**ファイル**: `packages/foundry/script/DeployTestnet.s.sol`

```solidity
contract DeployTestnet is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("DEPLOYER_PRIVATE_KEY");
        vm.startBroadcast(deployerPrivateKey);
        
        WerewolfGame game = new WerewolfGame();
        WerewolfProofVerifier verifier = new WerewolfProofVerifier();
        WerewolfRewards rewards = new WerewolfRewards(address(game));
        
        game.setVerifier(address(verifier));
        game.setRewardsContract(address(rewards));
        
        vm.stopBroadcast();
        
        // デプロイ情報をログ出力
        console.log("WerewolfGame deployed to:", address(game));
        console.log("WerewolfProofVerifier deployed to:", address(verifier));
        console.log("WerewolfRewards deployed to:", address(rewards));
    }
}
```

**実行**:
```bash
# Sepolia テストネット
yarn deploy --file DeployTestnet.s.sol --network sepolia
```

**フロントエンド設定更新**:
```typescript
// packages/nextjs/scaffold.config.ts
const scaffoldConfig = {
  targetNetworks: [chains.sepolia], // ローカル検証時は chains.foundry
  // ...
};
```

---

### Phase 5: 報酬システム実装（+α）

#### 5.1 参加費・賞金プールの実装

**スマートコントラクト拡張**: `packages/foundry/contracts/WerewolfRewards.sol`

```solidity
function joinGameWithFee(bytes32 gameId) external payable {
    require(msg.value >= gamePools[gameId].entryFee, "Insufficient entry fee");
    
    gamePools[gameId].totalAmount += msg.value;
    
    emit RewardDeposited(gameId, msg.sender, msg.value);
}
```

#### 5.2 勝利報酬の自動分配

```solidity
function distributeRewards(bytes32 gameId, address[] calldata winners) external onlyGameContract {
    RewardPool storage pool = gamePools[gameId];
    require(pool.totalAmount > 0, "No rewards to distribute");
    
    uint256 totalWinners = winners.length;
    require(totalWinners > 0, "No winners");
    
    uint256 rewardPerWinner = (pool.totalAmount * pool.winnerShare) / (10000 * totalWinners);
    
    for (uint256 i = 0; i < totalWinners; i++) {
        (bool success, ) = winners[i].call{value: rewardPerWinner}("");
        require(success, "Transfer failed");
        
        emit RewardClaimed(gameId, winners[i], rewardPerWinner);
    }
    
    emit RewardsDistributed(gameId, pool.totalAmount, totalWinners);
}
```

#### 5.3 フロントエンド連携

**ファイル**: `packages/nextjs/hooks/useRewards.ts`

```typescript
export const useRewards = (gameId: string) => {
  const { address } = useAccount();
  const { writeContractAsync } = useScaffoldWriteContract({ contractName: "WerewolfRewards" });

  const joinWithFee = async (entryFee: bigint) => {
    return await writeContractAsync({
      functionName: "joinGameWithFee",
      args: [gameId],
      value: entryFee,
    });
  };

  const claimReward = async () => {
    return await writeContractAsync({
      functionName: "claimReward",
      args: [gameId],
    });
  };

  return { joinWithFee, claimReward };
};
```

---

## 実装タスクリスト

### 必須タスク

#### スマートコントラクト
- [ ] WerewolfGame.sol 実装
- [ ] WerewolfProofVerifier.sol 実装
- [ ] WerewolfRewards.sol 実装
- [ ] デプロイスクリプト作成
- [ ] ユニットテスト作成

#### バックエンド
- [ ] ブロックチェーンクライアントモジュール実装
- [ ] ゲーム状態ハッシュ計算実装
- [ ] ゲームライフサイクルへの統合
- [ ] コミットメント連携実装
- [ ] 証明検証連携実装
- [ ] 環境変数・設定追加

#### フロントエンド
- [ ] useGameContract フック実装
- [ ] useCommitmentSubmission フック実装
- [ ] 試合履歴ページ実装
- [ ] 試合詳細ページ実装
- [ ] ExplorerLinks コンポーネント実装
- [ ] 既存ゲームUIへの統合

#### テスト・デプロイ
- [ ] ローカル環境での統合テスト
- [ ] Anvil上での動作確認
- [ ] Block Explorerでの確認機能

### オプショナルタスク

#### テストネット対応
- [ ] Sepoliaデプロイスクリプト
- [ ] テストネットでの動作確認
- [ ] Etherscan連携

#### 報酬システム
- [ ] 参加費徴収機能
- [ ] 賞金プール管理
- [ ] 勝利報酬分配
- [ ] UI実装

---

## セキュリティ考慮事項

### スマートコントラクト
1. **アクセス制御**: オーナー/ゲームマスターのみが特定の関数を呼べるようにする
2. **再入攻撃対策**: ReentrancyGuard の使用
3. **整数オーバーフロー**: Solidity 0.8+ の自動チェック
4. **ガス最適化**: 不要なストレージ書き込みを削減

### バックエンド
1. **秘密鍵管理**: 環境変数での管理、本番ではAWS Secrets Manager使用
2. **RPC認証**: Alchemy/Infura APIキーの保護
3. **トランザクション検証**: チェーン上の状態と整合性確認
4. **レート制限**: ブロックチェーン呼び出しの制限

### フロントエンド
1. **ウォレット接続**: RainbowKitのセキュアな実装
2. **トランザクション署名**: ユーザーへの明確な確認UI
3. **コントラクトアドレス検証**: `deployedContracts.ts` / `useDeployedContractInfo` を参照し、ハードコードを避ける

---

## パフォーマンス最適化

1. **バッチ処理**: 複数のコミットメントをまとめて処理
2. **イベントログ活用**: チェーン状態の効率的な取得
3. **キャッシング**: オフチェーンでの状態キャッシュ
4. **非同期処理**: ブロックチェーン呼び出しの非同期化

---

## モニタリング・ロギング

1. **トランザクション追跡**: 各トランザクションのハッシュをログ
2. **ガス使用量監視**: コスト最適化のための分析
3. **エラーハンドリング**: ブロックチェーンエラーの適切な処理とロギング
4. **アラート**: 異常なトランザクション失敗率の検知

---

## 今後の拡張性

1. **Layer 2統合**: Arbitrum/Optimismへの対応
2. **NFT報酬**: 勝利者へのNFT発行
3. **DAO統合**: ゲームルール変更の投票メカニズム
4. **クロスチェーン**: 複数チェーン対応
5. **トーナメント**: 複数ゲームをまたぐ大会システム

---

## 参考資料

- [Scaffold-ETH 2 Docs](https://docs.scaffoldeth.io/)
- [Foundry Book](https://book.getfoundry.sh/)
- [alloy Documentation](https://alloy.rs/)
- [alloy crates (docs.rs)](https://docs.rs/alloy/latest/alloy/)
- [wagmi Documentation](https://wagmi.sh/)
- [OpenZeppelin Contracts](https://docs.openzeppelin.com/contracts/)
- [ZK-SNARK Verifier Template](https://github.com/iden3/snarkjs)

---

## 実装スケジュール（例）

| Phase | タスク | 期間 |
|-------|--------|------|
| Phase 1 | スマートコントラクト実装 | 1週間 |
| Phase 2 | バックエンド統合 | 1週間 |
| Phase 3 | フロントエンド統合 | 1週間 |
| Phase 4 | テスト・デバッグ | 3日 |
| Phase 5 | 報酬システム（オプション） | 4日 |

**合計**: 約3〜4週間
