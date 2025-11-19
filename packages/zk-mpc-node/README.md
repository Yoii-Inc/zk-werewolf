# ZK-MPC-Node

鍵ペアの生成
```bash
# ノード0の鍵を生成(例)
cargo run --release keygen --id 0

# ノード1の鍵を生成(例)
cargo run --release keygen --id 1
```

ノードの起動
```bash
# ノード0を起動(例)
cargo run --release start --id 0 --input ./address/3
cargo run --release start --id 0 --input ./address/localhost3

# ノード1を起動(例)
cargo run --release start --id 1 --input ./address/3
```

インテグレーションテスト

```bash
cargo test test_mpc_node_proof_generation --test integration_test -- --nocapture
cargo test --release test_mpc_node_proof_generation --test integration_test -- --nocapture
```

curl で証明のリクエスト

```bash
curl -X POST http://localhost:9000 \
-H "Content-Type: application/json" \
-d '{
  "circuit_type": {
    "Built": "MySimpleCircuit"
  },
  "inputs": {
    "Built": {
      "MySimpleCircuit": {
        "a": 2,
        "b": 3
      }
    }
  }
}'
```

curl で証明の問い合わせ

```bash
curl -X GET http://localhost:9000/proof/YOUR_PROOF_ID
```

## API Endpoints

The server provides REST API endpoints for submitting proof requests, checking proof generation status, and retrieving proof outputs.

```
POST /
GET /proof/{proof_id}
GET /proof/{proof_id}/output
```

### Submit Proof Request

Submit a new proof generation request.

```

```

**Request Body:**
```json
{
    "proof_id": string,
    // Additional proof parameters
}
```

**Response:**
```json
{
    "success": true,
    "message": "Request accepted successfully"
}
```

### Get Proof Status

Get the current status of a proof generation request.

```
GET /proof/{proof_id}
```

**Response:**
```json
{
    "state": string,      // "pending", "processing", "completed", "failed"
    "proof_id": string,
    "message": string?,   // Optional status message
    "output": object?     // Optional proof output
}
```

### Get Proof Output

Get the output of a completed proof.

```
GET /proof/{proof_id}/output
```

**Response:**
```json
{
    // Proof output data
}
```

**Error Response:**
```json
{
    // Proof output data (error case)
}
```