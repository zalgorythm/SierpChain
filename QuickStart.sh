#!/bin/bash

# ==============================================================================
# QuickStart Script for SierpChain
# ==============================================================================
#
# This script automates the setup of a local multi-node SierpChain network.
#
# Commands:
#   start   - Builds the project and starts a network of nodes.
#   stop    - Stops all running SierpChain nodes.
#   logs    - Tails the logs of all running nodes.
#
# ==============================================================================

set -euo pipefail

# Configuration
# ------------------------------------------------------------------------------
NUM_PEERS=3
BASE_HTTP_PORT=8080
BASE_P2P_PORT=10000
LOG_DIR="logs"
BINARY_PATH="./target/release/sierpchain"

# Functions
# ------------------------------------------------------------------------------

# Stop all SierpChain processes
stop_nodes() {
    echo "🛑 Stopping all SierpChain nodes..."
    pkill -f "$BINARY_PATH" || echo "No nodes were running."
    rm -rf "$LOG_DIR"
    rm -f bootstrap_node_info.txt
}

# Start the multi-node network
start_nodes() {
    echo "🚀 Starting SierpChain network..."

    # 1. Clean up previous runs
    stop_nodes
    mkdir -p "$LOG_DIR"

    # 2. Build the project (assuming it's already built)
    if [ ! -f "$BINARY_PATH" ]; then
        echo "❌ Binary not found. Please run 'cargo build --release' first."
        exit 1
    fi

    # 3. Start Bootstrap Node
    echo "🌐 Starting bootstrap node..."
    local bootstrap_http_port=$BASE_HTTP_PORT
    local bootstrap_p2p_port=$BASE_P2P_PORT
    local bootstrap_log="$LOG_DIR/bootstrap.log"

    $BINARY_PATH --http-port "$bootstrap_http_port" --p2p-port "$bootstrap_p2p_port" > "$bootstrap_log" 2>&1 &
    BOOTSTRAP_PID=$!
    echo "Bootstrap node started with PID $BOOTSTRAP_PID. Logs at $bootstrap_log"

    # 4. Wait for Bootstrap Node to be ready and get its Peer ID
    echo "⏳ Waiting for bootstrap node to be ready..."
    local peer_id=""
    while [ -z "$peer_id" ]; do
        if ! ps -p $BOOTSTRAP_PID > /dev/null; then
            echo "❌ Bootstrap node failed to start. Check logs at $bootstrap_log"
            exit 1
        fi
        peer_id=$(grep "Peer ID" "$bootstrap_log" | awk '{print $NF}')
        sleep 1
    done
    local bootstrap_multiaddr="/ip4/127.0.0.1/tcp/${bootstrap_p2p_port}/p2p/${peer_id}"
    echo "✅ Bootstrap node is ready. Multiaddress: $bootstrap_multiaddr"
    echo "$bootstrap_multiaddr" > bootstrap_node_info.txt

    # 5. Start Peer Nodes
    for i in $(seq 1 "$NUM_PEERS"); do
        local http_port=$((BASE_HTTP_PORT + i))
        local p2p_port=$((BASE_P2P_PORT + i))
        local log_file="$LOG_DIR/peer_${i}.log"

        echo "starting peer ${i}..."
        $BINARY_PATH --http-port "$http_port" --p2p-port "$p2p_port" --peer "$bootstrap_multiaddr" > "$log_file" 2>&1 &
        PEER_PID=$!
        echo "🔗 Started Peer $i with PID $PEER_PID. HTTP: $http_port, P2P: $p2p_port. Logs: $log_file"
    done

    echo "✅ All nodes are running. Network is up!"
    echo "Use './QuickStart.sh logs' to see the logs."
    echo "Use './QuickStart.sh stop' to stop the network."
}

# Tail logs of all nodes
tail_logs() {
    if [ ! -d "$LOG_DIR" ]; then
        echo "❌ Log directory not found. Are the nodes running?"
        exit 1
    fi
    echo "📜 Tailing logs... (Press Ctrl+C to stop)"
    tail -f "$LOG_DIR"/*.log
}


# Main script logic
# ------------------------------------------------------------------------------
COMMAND=${1:-"help"}

case "$COMMAND" in
    start)
        start_nodes
        ;;
    stop)
        stop_nodes
        ;;
    logs)
        tail_logs
        ;;
    *)
        echo "SierpChain QuickStart Script"
        echo "--------------------------"
        echo "Usage: $0 {start|stop|logs}"
        echo "  start - Build and start a local test network."
        echo "  stop  - Stop all running nodes."
        echo "  logs  - View the logs of all running nodes."
        exit 1
        ;;
esac

exit 0
