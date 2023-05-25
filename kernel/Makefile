override MAKEFLAGS += -rR

# Default target.
.PHONY: all
all:
	RUSTFLAGS="-Crelocation-model=static -Cforce-frame-pointers=true" cargo build --target x86_64-unknown-none --release
	cp target/x86_64-unknown-none/release/kernel kernel.elf

# Remove object files and the final executable.
.PHONY: clean
clean:
	cargo clean
	rm -rf kernel.elf
