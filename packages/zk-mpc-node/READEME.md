# ZK-MPC-Node

サーバーの起動

```bash
cargo run --release 0 ./address/3
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
