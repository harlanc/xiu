all: build

# Build the checked-out local source tree. No manifest replacement is needed.
check:
	cargo clippy --fix --allow-dirty --allow-no-vcs
clean:
	cargo clean
build:
	cargo build
