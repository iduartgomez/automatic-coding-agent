# ACA Development Environment Base Image
#
# This image contains:
# - Claude Code CLI
# - Common development tools (git, nodejs, python, rust)
# - The aca binary
# - All necessary dependencies for running automated coding tasks

FROM ubuntu:22.04

# Avoid interactive prompts during package installation
ENV DEBIAN_FRONTEND=noninteractive

# Set up locale
ENV LANG=C.UTF-8
ENV LC_ALL=C.UTF-8

# Install base dependencies
RUN apt-get update && apt-get install -y \
    # Base tools
    curl \
    wget \
    git \
    ca-certificates \
    gnupg \
    lsb-release \
    build-essential \
    pkg-config \
    libssl-dev \
    # Shell and utilities
    bash \
    zsh \
    vim \
    nano \
    jq \
    unzip \
    zip \
    # Network tools
    netcat \
    telnet \
    iputils-ping \
    dnsutils \
    && rm -rf /var/lib/apt/lists/*

# Install Node.js 20.x LTS
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y nodejs \
    && npm install -g npm@latest \
    && npm install -g yarn pnpm \
    && rm -rf /var/lib/apt/lists/*

# Install Python 3.11+ with pip
RUN apt-get update && apt-get install -y \
    python3 \
    python3-pip \
    python3-venv \
    python3-dev \
    && rm -rf /var/lib/apt/lists/* \
    && python3 -m pip install --upgrade pip setuptools wheel

# Install Rust toolchain
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable \
    && . "$HOME/.cargo/env" \
    && rustup component add clippy rustfmt

# Add Rust to PATH for all users
ENV PATH="/root/.cargo/bin:${PATH}"

# Install Docker CLI (for potential nested Docker usage)
RUN apt-get update && apt-get install -y \
    apt-transport-https \
    && curl -fsSL https://download.docker.com/linux/ubuntu/gpg | gpg --dearmor -o /usr/share/keyrings/docker-archive-keyring.gpg \
    && echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/docker-archive-keyring.gpg] https://download.docker.com/linux/ubuntu $(lsb_release -cs) stable" | tee /etc/apt/sources.list.d/docker.list > /dev/null \
    && apt-get update \
    && apt-get install -y docker-ce-cli \
    && rm -rf /var/lib/apt/lists/*

# Install Go (useful for many projects)
RUN wget https://go.dev/dl/go1.22.0.linux-amd64.tar.gz \
    && tar -C /usr/local -xzf go1.22.0.linux-amd64.tar.gz \
    && rm go1.22.0.linux-amd64.tar.gz

ENV PATH="/usr/local/go/bin:${PATH}"

# Install Claude Code CLI
# Note: This assumes claude-cli will be available. Adjust the installation method based on actual availability
RUN curl -fsSL https://claude.ai/install.sh | bash || \
    echo "Claude CLI installation requires manual setup - see https://claude.ai/code"

# Create workspace directory
RUN mkdir -p /workspace
WORKDIR /workspace

# Set up non-root user for security
RUN useradd -m -s /bin/bash -u 1000 aca-user \
    && usermod -aG sudo aca-user \
    && echo "aca-user ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers

# Copy aca binary (will be provided during build)
# This is optional - the binary can also be mounted as a volume
COPY --chown=aca-user:aca-user aca /usr/local/bin/aca 2>/dev/null || true
RUN chmod +x /usr/local/bin/aca 2>/dev/null || true

# Set up environment
ENV HOME=/home/aca-user
ENV USER=aca-user

# Install common language-specific tools
USER aca-user
WORKDIR /home/aca-user

# Install common npm packages
RUN npm install -g \
    typescript \
    ts-node \
    eslint \
    prettier \
    jest \
    webpack \
    vite

# Install common Python packages
RUN python3 -m pip install --user \
    black \
    pylint \
    pytest \
    mypy \
    poetry \
    requests \
    numpy \
    pandas

# Install Rust dev tools
RUN rustup component add rust-analyzer

# Back to root for final setup
USER root

# Set up entrypoint
COPY --chown=root:root entrypoint.sh /usr/local/bin/entrypoint.sh 2>/dev/null || echo '#!/bin/bash\nexec "$@"' > /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

# Default to aca-user
USER aca-user
WORKDIR /workspace

ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
CMD ["/bin/bash"]
