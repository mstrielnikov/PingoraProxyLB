#!/usr/bin/env bash
set -e

# This sandboxing script safely loads the Aya-ebpf payload inside an isolated network
# namespace. This guarantees we don't accidentally drop all packets on the host if the
# BPF program malfunctions. Requires root execution to instantiate netns.

if [ "$EUID" -ne 0 ]; then
  echo "Please run as root (or via sudo)"
  exit 1
fi

NS="clb-sandbox"
VETH0="veth-host"
VETH1="veth-guest"

echo "[1/4] Setting up Network Namespace: $NS"
ip netns delete $NS 2>/dev/null || true
ip netns add $NS

echo "[2/4] Linking Virtual Ethernet interfaces"
ip link add $VETH0 type veth peer name $VETH1
ip link set $VETH1 netns $NS

# Bring interfaces up
ip link set $VETH0 up
ip -n $NS link set $VETH1 up
ip -n $NS link set lo up

# Assign IPs
ip addr add 10.200.1.1/24 dev $VETH0
ip -n $NS addr add 10.200.1.2/24 dev $VETH1

echo "[3/4] Readying test instance..."
# At this point, inside the namespace you can run the eBPF pingora application:
# ip netns exec $NS cargo run --example static_dispatch -- --interface veth-guest

echo "[4/4] Interactive Sandbox Terminal (Type 'exit' to dismantle)"
ip netns exec $NS bash

echo "Cleaning up..."
ip link delete $VETH0 2>/dev/null || true
ip netns delete $NS
echo "Sandbox destroyed."
