#!/bin/bash
TRACE_LEVEL=${TRACE_LEVEL:-trace}
export RUST_LOG=agent_diva_agent=$TRACE_LEVEL
echo "Starting agent-diva gateway with trace level: $TRACE_LEVEL"
cargo run -- gateway
