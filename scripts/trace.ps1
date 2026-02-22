param (
    [string]$TraceLevel = "trace"
)

if ($env:TRACE_LEVEL) {
    $TraceLevel = $env:TRACE_LEVEL
}

$env:RUST_LOG = "agent_diva_agent=$TraceLevel"
Write-Host "Starting agent-diva gateway with trace level: $TraceLevel"
cargo run -- gateway
