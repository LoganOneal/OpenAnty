#!/usr/bin/env bash
# Open Anty portable first-run helper (macOS / Linux)
set -euo pipefail
DIR="$(cd "$(dirname "$0")" && pwd)"
export PATH="$DIR/bin:$PATH"
echo "Initializing Open Anty..."
openanty init
echo ""
echo "Doctor:"
openanty doctor
echo ""
echo "MCP config:"
openanty mcp-config
echo ""
echo "Done. Add bin/ to PATH or invoke bin/openanty directly."
