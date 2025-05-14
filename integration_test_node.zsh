#!/bin/bash
set -ex
trap "exit" INT TERM
trap 'jobs -p | xargs -r kill' EXIT

MAX_WAIT=60 # 最大待機時間（秒）

# ポートが開くのを待つ関数
wait_for_port() {
    local port=$1
    local start_time=$(date +%s)
    
    while ! curl -s http://localhost:${port} > /dev/null; do
        if [ $(($(date +%s) - start_time)) -gt $MAX_WAIT ]; then
            echo "タイムアウト: ポート ${port} が開きませんでした"
            exit 1
        fi
        sleep 1
    done
}

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
./target/release/zk-mpc-node 1 ./address/3 > /dev/null &
NODE1_PID=$!
./target/release/zk-mpc-node 2 ./address/3 > /dev/null &
NODE2_PID=$!


# ノードの起動を待機
for PORT in 9000 9001 9002; do
  wait_for_port $PORT
done

# インテグレーションテストの実行
cargo test --test integration_test -- --nocapture --test-threads=1

# 全プロセスの終了
kill $SERVER_PID $NODE0_PID $NODE1_PID $NODE2_PID

echo "Integration tests(zk-mpc-node) completed"