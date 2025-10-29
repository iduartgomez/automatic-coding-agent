# ACA Container Image

This directory contains the Dockerfile and scripts for building the ACA development environment base image.

## What's Included

The `aca-dev:latest` image includes:

- **Ubuntu 22.04 LTS** base
- **Claude Code CLI** - AI-powered coding assistant
- **Node.js 20.x LTS** - JavaScript/TypeScript runtime with npm, yarn, pnpm
- **Python 3.11+** - Python interpreter with pip and common packages
- **Rust stable** - Rust toolchain with cargo, clippy, rustfmt
- **Go 1.22** - Go programming language
- **Docker CLI** - For potential nested Docker usage
- **Git** - Version control
- **Development tools** - vim, nano, jq, curl, wget, etc.

## Building the Image

### From Rust Code

```rust
use aca::container::ImageBuilder;
use bollard::Docker;

let docker = Docker::connect_with_local_defaults()?;
let builder = ImageBuilder::new(docker);

// Build from this directory
let image_tag = builder.build_aca_base_image(
    Path::new("container"),
    None  // Uses default tag "aca-dev:latest"
).await?;
```

### From Command Line

```bash
# From project root
docker build -t aca-dev:latest -f container/Dockerfile container/

# Or using the orchestrator (recommended)
cargo run -- build-image
```

## Using the Image

```rust
use aca::container::{ContainerOrchestrator, ContainerConfig, ACA_BASE_IMAGE};

let orchestrator = ContainerOrchestrator::new().await?;

let config = ContainerConfig::builder()
    .image(ACA_BASE_IMAGE)  // Uses "aca-dev:latest"
    .working_dir("/workspace")
    .bind("/path/to/project:/workspace:rw")
    .build()?;

let container_id = orchestrator.create_container(&config, None).await?;
orchestrator.start_container(&container_id).await?;
```

## Image Layers

1. **Base Ubuntu 22.04**
2. **System dependencies** - ca-certificates, curl, build tools
3. **Node.js ecosystem** - Node, npm, yarn, pnpm, common packages
4. **Python ecosystem** - Python, pip, common packages
5. **Rust ecosystem** - rustup, cargo, tools
6. **Go ecosystem** - Go compiler and tools
7. **Docker CLI** - For nested Docker operations
8. **Claude Code** - AI coding assistant
9. **ACA binary** (optional) - Pre-built aca binary
10. **User setup** - Non-root `aca-user` with sudo access

## Directory Structure

```
container/
├── Dockerfile       # Image definition
├── entrypoint.sh    # Container startup script
├── README.md        # This file
└── aca              # (Generated) aca binary to include in image
```

## Building with ACA Binary

To include the `aca` binary in the image:

```bash
# Build the aca binary first
cargo build --release

# Copy to container directory
cp target/release/aca container/

# Build the image
docker build -t aca-dev:latest -f container/Dockerfile container/
```

## Customization

### Adding More Tools

Edit `Dockerfile` to add additional tools:

```dockerfile
# Install additional language
RUN apt-get update && apt-get install -y \
    ruby \
    ruby-dev \
    && gem install bundler
```

### Changing Base Image

Change the first line to use a different base:

```dockerfile
FROM ubuntu:24.04  # or debian:bookworm, etc.
```

### Pre-installing Project Dependencies

You can create a derived image with project-specific dependencies:

```dockerfile
FROM aca-dev:latest

# Install project-specific npm packages
COPY package.json package-lock.json ./
RUN npm ci

# Install project-specific Python packages
COPY requirements.txt ./
RUN pip install -r requirements.txt
```

## Security Considerations

- Container runs as non-root `aca-user` by default
- User has sudo access (password-less) for installing additional tools
- For production, consider removing sudo access or running fully unprivileged

## Image Size

The full image is approximately **3-4 GB** due to including multiple language ecosystems.

To reduce size:
- Remove unused languages from Dockerfile
- Use multi-stage builds
- Clean up package caches more aggressively

## Troubleshooting

### Claude CLI Not Available

The Claude CLI installation may require manual setup. If the automated installation fails:

1. Visit https://claude.ai/code
2. Follow installation instructions
3. Rebuild image or install in running container

### Build Fails

```bash
# Check Docker is running
docker info

# Check available disk space
df -h

# Clean up old images
docker image prune -a
```

### Permissions Issues

Ensure the entrypoint script is executable:

```bash
chmod +x container/entrypoint.sh
```
