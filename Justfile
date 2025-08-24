# List all available recipes
@default:
    just --list

# Format all Rust code
fmt:
    cargo fmt --all

# Check formatting without making changes
fmt-check:
    cargo fmt --all -- --check

# Run Rust linter
lint:
    cargo clippy -- -D warnings

# Run tests
test:
    cargo test

# Build the project
build:
    cargo build

# Build for release
build-release:
    cargo build --release

# Clean build artifacts
clean:
    cargo clean

# Install required targets
install-targets:
    rustup target add x86_64-pc-windows-gnu
    rustup target add x86_64-unknown-linux-musl

# Switch to deployment configuration (Linux musl)
switch-to-deploy: install-targets
    @echo "Switching to deployment configuration..."
    cp .cargo/config.toml .cargo/config.local.toml
    echo "[build]\ntarget = \"x86_64-unknown-linux-musl\"\n\n[target.x86_64-unknown-linux-musl]\nrustflags = [\"-C\", \"target-feature=+crt-static\"]" > .cargo/config.toml

# Switch to local development configuration (Windows GNU)
switch-to-local:
    @echo "Switching to local configuration..."
    @if [ -f .cargo/config.local.toml ]; then \
        cp .cargo/config.local.toml .cargo/config.toml; \
    else \
        echo "[build]\ntarget = \"x86_64-pc-windows-gnu\"\n\n[target.x86_64-pc-windows-gnu]\nlinker = \"gcc\"\nar = \"gcc-ar\"" > .cargo/config.toml; \
    fi

# Run development server locally
dev: switch-to-local
    @echo "Starting local development server..."
    AWS_LAMBDA_FUNCTION_NAME="handler" \
    AWS_LAMBDA_RUNTIME_API="localhost" \
    AWS_REGION="local" \
    RUST_LOG="info" \
    cargo run --bin handler

# Run Vercel development server
vercel-dev: switch-to-local
    vercel dev

# Deploy to Vercel preview environment
deploy-preview: switch-to-deploy
    @echo "Deploying with musl target..."
    vercel
    just switch-to-local

# Deploy to Vercel production
deploy-prod: switch-to-deploy
    @echo "Deploying to production with musl target..."
    vercel --prod
    just switch-to-local

# Build for Vercel deployment (prebuilt)
vercel-build:
    vercel build

# Deploy prebuilt project
deploy-prebuilt:
    vercel deploy --prebuilt

# Build and deploy prebuilt project
deploy-prebuilt-all: vercel-build deploy-prebuilt
    echo "Prebuilt deployment completed!"

# Check for outdated dependencies
outdated:
    cargo outdated

# Update dependencies
update:
    cargo update

# Install Vercel CLI if not already installed
install-vercel:
    npm install -g vercel

# Setup cross-compilation tools (Windows only)
setup-cross-compile:
    @echo "Installing cross-compilation tools..."
    @echo "Please run these commands in an administrator PowerShell:"
    @echo "choco install mingw"
    @echo "choco install gcc-arm"

# Run all checks (format, lint, test)
check: fmt-check lint test
    echo "All checks passed!"

# Watch for changes and run tests
watch-test:
    cargo watch -x test

# Watch for changes and run the development server
watch-dev: switch-to-local
    @echo "Starting local development server..."
    AWS_LAMBDA_FUNCTION_NAME="handler" \
    AWS_LAMBDA_RUNTIME_API="localhost" \
    AWS_REGION="local" \
    RUST_LOG="info" \
    cargo watch -qcw ./api -s "vercel dev"
