py := "python/refix-engine"

# List available recipes
default:
    just --list

# Install or refresh the Python dev environment
sync:
    uv sync --project {{ py }}

# Compile and install the extension module into the venv
develop:
    cd {{ py }} && uv run maturin develop --uv

# Run the Python test suite
pytest *args:
    uv run --project {{ py }} pytest {{ args }}

# Run the Rust test suite
cargo-test:
    cargo test

# Run all tests
test: cargo-test pytest

# Format Rust code
fmt:
    cargo fmt --all

# Check formatting and lints without modifying anything
lint:
    cargo fmt --all -- --check
    cargo clippy --all-targets
    cargo fmt --manifest-path crates/refix-message/fuzz/Cargo.toml --all -- --check
    cargo clippy --manifest-path crates/refix-message/fuzz/Cargo.toml --all-targets

# Type-check the Python layer
typecheck:
    cd {{ py }} && uv run pyright

# Build a release wheel
wheel:
    cd {{ py }} && uv run maturin build --release
