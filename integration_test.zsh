#!/bin/bash
set -ex
trap "exit" INT TERM
trap 'jobs -p | xargs -r kill' EXIT

MAX_WAIT=60 # 最大待機時間（秒）

# ポートが開くのを待つ関数
wait_for_port() {
    local port=$1
    local start_time=$(date +%s)

    while ! curl -s http://localhost:${port} >/dev/null; do
        if [ $(($(date +%s) - start_time)) -gt $MAX_WAIT ]; then
            echo "タイムアウト: ポート ${port} が開きませんでした"
            exit 1
        fi
        sleep 1
    done
}

run_crypto_tests() {
    echo "Running crypto integration tests..."
    # Create test data directory
    mkdir -p test-data

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
    ./target/release/server &
    SERVER_PID=$!

    # サーバーの起動を待機
    wait_for_port 8080

    # zk-mpc-nodeのビルド
    cd ../zk-mpc-node
    cargo build --release

    # 3つのノードを起動（それぞれ異なるポートで）
    ./target/release/zk-mpc-node 0 ./address/3 &
    NODE0_PID=$!
    ./target/release/zk-mpc-node 1 ./address/3 >/dev/null &
    NODE1_PID=$!
    ./target/release/zk-mpc-node 2 ./address/3 >/dev/null &
    NODE2_PID=$!

    # ノードの起動を待機
    for PORT in 9000 9001 9002; do
        wait_for_port $PORT
    done

    # zk-mpc-node インテグレーションテストの実行
    cargo test --test integration_test -- --nocapture --test-threads=1

    # server インテグレーションテストの実行
    cd ../server
    cargo test --test "*" -- --nocapture --test-threads=1
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
