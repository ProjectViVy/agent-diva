#!/usr/bin/env bash
# Reproducible standalone framework checks; does not build a product installer.
set -euo pipefail
cd -- "$(dirname -- "$0")"
case "${1:-}" in ""|--native) ;; *) echo 'usage: verify.sh [--native]' >&2; exit 2;; esac
go version
go env GOOS GOARCH CGO_ENABLED
test -z "$(gofmt -l ./*.go)"
go mod download
go mod verify
go vet ./...
go test -count=1 ./...
go test -race -count=1 ./...
CGO_ENABLED=0 go test -count=1 ./...
mkdir -p build
go build -o build/probe-default .
CGO_ENABLED=0 go build -o build/probe-linux-nocgo .
GOOS=windows GOARCH=amd64 CGO_ENABLED=0 go build -o build/probe-windows.exe .
go run github.com/egoist/mygo/cmd/mygo generate
npm ci --ignore-scripts --no-audit --no-fund
npm run typecheck
if [[ "${1:-}" == --native ]]; then
  MYGO_PROBE_NATIVE=1 go test -count=1 -v -run '^TestNative' -timeout 60s ./...
fi
