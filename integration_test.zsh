#!/bin/bash
set -ex
trap "exit" INT TERM
trap 'jobs -p | xargs -r kill' EXIT

SERVER_STARTUP_TIMEOUT=${SERVER_STARTUP_TIMEOUT:-60}   # サーバー待機秒
NODE_STARTUP_TIMEOUT=${NODE_STARTUP_TIMEOUT:-300}      # ノード待機秒（PK読込を考慮）

# ポートが開くのを待つ関数
wait_for_port() {
    local port=$1
    local timeout=$2
    local label=$3
    local pid=$4
    local start_time=$(date +%s)

    while ! curl -s http://localhost:${port} >/dev/null; do
        if [ -n "${pid}" ] && ! kill -0 "${pid}" 2>/dev/null; then
            echo "失敗: ${label} プロセスが起動中に終了しました（PID=${pid}）"
            exit 1
        fi
        if [ $(($(date +%s) - start_time)) -gt "${timeout}" ]; then
            echo "タイムアウト: ${label} (${port}) が ${timeout}秒以内に起動しませんでした"
            exit 1
        fi
        sleep 1
    done
}

run_crypto_tests() {
    echo "Running crypto integration tests..."
    # Create test data directory
    mkdir -p test-data

    yarn install

    # Run crypto tests
    cd ./packages/zk-mpc-node
    cargo test --test crypto_integration_test -- --nocapture

    # Cleanup
    cd ../../
    rm -rf test-data
}

run_server_node_tests() {
    # サーバーのビルドと起動（release mode）
    cd ./packages/server
    cargo build --release
    ./../../target/release/server &
    SERVER_PID=$!

    # サーバーの起動を待機
    wait_for_port 8080 "${SERVER_STARTUP_TIMEOUT}" "server" "${SERVER_PID}"

    # zk-mpc-nodeのビルド
    cd ../zk-mpc-node
    cargo build --release

    # 鍵ペアの生成
    cargo run --release keygen --id 0
    cargo run --release keygen --id 1
    cargo run --release keygen --id 2

    # 3つのノードを起動（それぞれ異なるポートで）
    ./../../target/release/zk-mpc-node start --id 0 &
    NODE0_PID=$!
    ./../../target/release/zk-mpc-node start --id 1 &
    NODE1_PID=$!
    ./../../target/release/zk-mpc-node start --id 2 &
    NODE2_PID=$!

    # ノードの起動を待機
    wait_for_port 9000 "${NODE_STARTUP_TIMEOUT}" "zk-mpc-node#0" "${NODE0_PID}"
    wait_for_port 9001 "${NODE_STARTUP_TIMEOUT}" "zk-mpc-node#1" "${NODE1_PID}"
    wait_for_port 9002 "${NODE_STARTUP_TIMEOUT}" "zk-mpc-node#2" "${NODE2_PID}"

    # zk-mpc-node インテグレーションテストの実行
    cargo test --test integration_test -- --nocapture --test-threads=1 || {
        echo "zk-mpc-node integration tests failed"
        exit 1
    }

    # server インテグレーションテストの実行
    cd ../server
    cargo test --test "*" -- --nocapture --test-threads=1 || {
        echo "server integration tests failed"
        exit 1
    }
}

main() {
    echo "Starting all tests..."

    # First run crypto tests (no server/node required)
    run_crypto_tests

    # Then run tests requiring server and nodes
    run_server_node_tests

    echo "All tests completed successfully"
}

main "$@"
