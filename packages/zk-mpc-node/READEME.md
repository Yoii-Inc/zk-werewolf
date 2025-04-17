# ZK-MPC-Node

サーバーの起動

```
cargo run --release 0 ./address/3
```

インテグレーションテスト
```
cargo test test_mpc_node_proof_generation --test integration_test -- --nocapture
cargo test --release test_mpc_node_proof_generation --test integration_test -- --nocapture
```

curlで証明のリクエスト
```
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

curlで証明の問い合わせ
```
curl -X GET http://localhost:9000/proof/YOUR_PROOF_ID
```