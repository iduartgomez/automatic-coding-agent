#!/bin/bash
set -e

# ACA Container Entrypoint
# Handles initialization and setup for the containerized environment

# Ensure .aca directory exists
mkdir -p /workspace/.aca

# Set up git config if not already configured
if [ ! -f ~/.gitconfig ]; then
    git config --global user.name "ACA Agent"
    git config --global user.email "agent@aca.local"
    git config --global init.defaultBranch main
fi

# Display environment info
echo "=== ACA Development Environment ==="
echo "Node.js: $(node --version)"
echo "npm: $(npm --version)"
echo "Python: $(python3 --version)"
echo "Rust: $(rustc --version 2>/dev/null || echo 'Not available')"
echo "Go: $(go version 2>/dev/null || echo 'Not available')"
echo "Claude Code: $(claude --version 2>/dev/null || echo 'Not available - manual setup required')"
echo "==================================="

# Execute the command
exec "$@"
