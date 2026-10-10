py := "python/refix-engine"
stock := "python/refix-fix44"

# List available recipes
default:
    just --list

# Install or refresh the Python dev environment
sync:
    uv sync --project {{ py }}

# Compile and install the extension module into the venv
develop:
    cd {{ py }} && uv run maturin develop --uv

# Run the Python test suite (benchmarks run once, untimed)
pytest *args:
    uv run --project {{ py }} pytest --benchmark-disable {{ args }}

# Run the Python benchmarks and save the results for comparison
pybench:
    cd {{ py }} && uv run pytest tests/test_benchmarks.py --benchmark-autosave

# Generate the stock packages' code and its fingerprint
generate-stock:
    cargo run -q -p refix-cli -- codegen dictionaries/quickfix/FIX44.xml \
        --rust crates/refix-fix44/src/generated.rs \
        --python python/refix-fix44/src/refix_fix44/__init__.py
    shasum -a 256 crates/refix-fix44/src/generated.rs \
        python/refix-fix44/src/refix_fix44/__init__.py > crates/refix-fix44/generated.sha256

# Check the stock packages as published
check-stock: generate-stock
    git diff --exit-code -- crates/refix-fix44/generated.sha256
    cargo clippy -p refix-fix44 --all-targets -- -D warnings
    cargo test -p refix-fix44
    rm -rf target/stock && uv build --wheel {{ stock }} -o target/stock
    cd {{ stock }} && uv run --project ../refix-engine pyright
    cd {{ stock }} && uv run --project ../refix-engine --with ../../target/stock/refix_fix44-*.whl python -m doctest README.md

# Check the CLI as published, through a wheel built from its sdist
check-cli:
    rm -rf target/cli && uv build python/refix-cli -o target/cli
    uvx --from target/cli/refix_cli-*.whl refix --version

# Run the Rust test suite
cargo-test:
    cargo test

# Run all tests
test: cargo-test pytest

# Run the Rust test suite with coverage (lcov output for codecov)
coverage-rust:
    cargo llvm-cov --lcov --output-path target/lcov.info

# Run the Python test suite with coverage (xml output for codecov)
coverage-python:
    cd {{ py }} && uv run pytest --benchmark-disable --cov=refix --cov-report=xml

# Format Rust and Python code
fmt:
    cargo fmt --all
    cd {{ py }} && uv run ruff format

# Check formatting and lints without modifying anything
lint:
    cargo fmt --all -- --check
    cargo clippy --all-targets
    cargo fmt --manifest-path fuzz/Cargo.toml --all -- --check
    cargo clippy --manifest-path fuzz/Cargo.toml --all-targets
    cd {{ py }} && uv run ruff format --check
    cd {{ py }} && uv run ruff check

# Type-check the Python layer
typecheck:
    cd {{ py }} && uv run pyright

# Build a release wheel
wheel:
    cd {{ py }} && uv run maturin build --release

# Run a fuzz target (requires nightly and cargo-fuzz)
fuzz target seeds time="60":
    mkdir -p fuzz/corpus/{{ target }}
    cargo +nightly fuzz run {{ target }} \
        fuzz/corpus/{{ target }} fuzz/{{ seeds }} -- -dict=fuzz/dict.txt -max_total_time={{ time }}
