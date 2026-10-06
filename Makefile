# oink - gamebooks on e-ink

.PHONY: all build run test check fmt lint clean m5paper docs docs-build docs-serve

all: build

build:
	cargo build --workspace

# Play the demo in the terminal. Pass flags with ARGS, e.g. make run ARGS="--seed 7"
run:
	cargo run -p oink-cli -- run examples/high-pass $(ARGS)

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
# Requires Node 20 or newer.
website/node_modules:
	cd website && npm install

docs: website/node_modules
	cd website && npm start

docs-build: website/node_modules
	cd website && npm run build

docs-serve: website/node_modules
	cd website && npm run serve

# M5Paper firmware player (planned as players/m5paper, built outside the workspace).
# Requires the espup toolchain: cargo install espup && espup install
m5paper:
	@echo "players/m5paper not created yet. See AGENTS.md."
