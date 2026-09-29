# oink - gamebooks on e-ink

.PHONY: all build sim test check fmt lint clean m5paper docs docs-build docs-serve

all: build

build:
	cargo build --workspace

# Run the desktop simulator (requires SDL2: brew install sdl2)
sim:
	cargo run -p oink-sim

test:
	cargo test --workspace

check:
	cargo check --workspace

fmt:
	cargo fmt --all

lint:
	cargo clippy --workspace -- -D warnings

clean:
	cargo clean

# Documentation site (Docusaurus in website/). Pages come from docs/.
# Requires Node; the version is pinned in website/.tool-versions.
website/node_modules:
	cd website && npm install

docs: website/node_modules
	cd website && npm start

docs-build: website/node_modules
	cd website && npm run build

docs-serve: website/node_modules
	cd website && npm run serve

# M5Paper firmware (not yet in the workspace).
# Requires the espup toolchain: cargo install espup && espup install
m5paper:
	@echo "oink-m5paper crate not created yet. See README."
