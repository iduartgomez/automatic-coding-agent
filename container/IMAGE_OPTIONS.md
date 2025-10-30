# Development Container Images

ACA supports multiple base images with different trade-offs:

## Available Images

### 1. **Ubuntu 22.04 Full** (Default: `aca-dev:latest`)
📦 **Size**: ~3-4 GB
🚀 **Build Time**: ~10-15 minutes
✅ **Best For**: Maximum compatibility, all tools work out-of-box

**Includes**:
- Ubuntu 22.04 LTS (glibc)
- Node.js 20.x + npm, yarn, pnpm + TypeScript, ESLint, Prettier, Jest
- Python 3.11+ + pip + black, pylint, pytest, mypy, poetry
- Rust stable + cargo, clippy, rustfmt, rust-analyzer
- Go 1.22
- Docker CLI
- Claude Code CLI (requires manual setup)
- Git + vim, nano, curl, wget, jq

**Build**:
```bash
docker build -t aca-dev:latest -f container/Dockerfile container/
```

---

### 2. **Alpine 3.19 Lightweight** (`aca-dev:alpine`)
📦 **Size**: ~800 MB-1 GB
🚀 **Build Time**: ~5-7 minutes
✅ **Best For**: Fast pulls, limited resources, simple projects

**Includes**:
- Alpine Linux 3.19 (musl libc)
- Node.js 20.x + npm, yarn, pnpm + TypeScript, ESLint, Prettier, Jest
- Python 3.11+ + pip + black, pylint, pytest
- Rust stable + cargo, clippy, rustfmt
- Go 1.22
- Docker CLI (static binary)
- Git + vim, nano, jq

**⚠️ Limitations**:
- Uses musl libc (some binaries may not work)
- Claude CLI may require glibc (manual workaround needed)
- Fewer pre-installed tools
- Some npm packages may have compilation issues

**Build**:
```bash
docker build -t aca-dev:alpine -f container/Dockerfile.alpine container/
```

---

## Comparison Table

| Feature | Ubuntu (Full) | Alpine (Lightweight) |
|---------|---------------|---------------------|
| **Size** | 3-4 GB | 800 MB-1 GB |
| **Build Time** | 10-15 min | 5-7 min |
| **Pull Time** | ~2-3 min | ~30-60 sec |
| **Libc** | glibc | musl |
| **Compatibility** | Excellent | Good |
| **Claude CLI** | ✅ Yes | ⚠️ Manual setup |
| **Pre-installed Tools** | Extensive | Minimal |
| **Package Manager** | apt | apk |
| **Best For** | Production | Development/CI |

---

## Other Lightweight Alternatives

### 3. **Debian Slim** (DIY)
📦 **Size**: ~1.5-2 GB
- Good middle ground between Ubuntu and Alpine
- glibc compatible (better than Alpine)
- Smaller than Ubuntu

```dockerfile
FROM debian:bookworm-slim
# Add your tools...
```

### 4. **Wolfi/Chainguard** (Modern minimal)
📦 **Size**: ~500 MB-1 GB
- Modern minimal base
- glibc compatible
- Built for security
- Requires manual image building

```dockerfile
FROM cgr.dev/chainguard/wolfi-base:latest
# Add your tools...
```

### 5. **Language-Specific Images**
For single-language projects, use official slim images:

- **Node.js**: `node:20-alpine` (~180 MB)
- **Python**: `python:3.11-slim` (~130 MB)
- **Rust**: `rust:1.75-slim` (~800 MB)
- **Go**: `golang:1.22-alpine` (~350 MB)

---

## Recommendations

### Use **Ubuntu Full** (`aca-dev:latest`) if:
- ✅ You need maximum compatibility
- ✅ Claude CLI must work out-of-box
- ✅ Disk space isn't constrained
- ✅ Running diverse projects (multi-language)
- ✅ You want "it just works"

### Use **Alpine** (`aca-dev:alpine`) if:
- ✅ You need faster container startup
- ✅ Bandwidth is limited (faster pulls)
- ✅ Running in CI/CD pipelines
- ✅ Simple Node.js/Python projects
- ✅ You're comfortable with musl libc quirks

### Use **Language-Specific** if:
- ✅ Single-language project
- ✅ Need minimal size
- ✅ Don't need Claude CLI in container
- ✅ Custom tooling requirements

---

## Switching Images

### In Code:
```rust
use aca::container::{ContainerConfig, ACA_BASE_IMAGE};

// Use default Ubuntu image
let config = ContainerConfig::builder()
    .image(ACA_BASE_IMAGE)  // "aca-dev:latest"
    .build()?;

// Or use Alpine
let config = ContainerConfig::builder()
    .image("aca-dev:alpine")
    .build()?;

// Or use official image
let config = ContainerConfig::builder()
    .image("node:20-alpine")
    .build()?;
```

### Build Both:
```bash
# Build full image
docker build -t aca-dev:latest -f container/Dockerfile container/

# Build Alpine image
docker build -t aca-dev:alpine -f container/Dockerfile.alpine container/

# List images
docker images | grep aca-dev
```

---

## Custom Images

You can create project-specific images:

```dockerfile
FROM aca-dev:latest

# Add project-specific dependencies
COPY package.json package-lock.json ./
RUN npm ci

# Pre-install Python packages
COPY requirements.txt ./
RUN pip install -r requirements.txt

# Your custom setup
RUN cargo install your-tool
```

Then use:
```rust
let config = ContainerConfig::builder()
    .image("my-custom-aca:latest")
    .build()?;
```

---

## Size Optimization Tips

1. **Multi-stage builds**: Build tools in one stage, runtime in another
2. **Clean caches**: `RUN apt-get clean && rm -rf /var/lib/apt/lists/*`
3. **Combine RUN commands**: Fewer layers = smaller image
4. **Use .dockerignore**: Don't copy unnecessary files
5. **Remove build dependencies**: Keep only runtime deps
6. **Use slim variants**: `node:20-slim` vs `node:20`

---

## Image Size Breakdown

**Ubuntu Full** (~3.5 GB):
- Base Ubuntu: ~80 MB
- Node.js ecosystem: ~500 MB
- Python ecosystem: ~400 MB
- Rust toolchain: ~1.5 GB
- Go: ~400 MB
- Tools + dependencies: ~620 MB

**Alpine** (~900 MB):
- Base Alpine: ~8 MB
- Node.js: ~150 MB
- Python: ~100 MB
- Rust: ~500 MB
- Go: ~130 MB
- Tools: ~12 MB
