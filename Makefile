export GIT_COMMIT=git.$(subst -,.,$(shell git describe --always --match=NeVeRmAtCh --dirty 2>/dev/null || git rev-parse --short HEAD 2>/dev/null))
export RUSTFLAGS
export CPU_PLATFORM

unexport FFI_USE_CUDA
unexport GPUPROXY_FEATURES

ifdef FFI_USE_CUDA
	GPUPROXY_FEATURES+=cuda
endif

ifneq ($(strip $(GPUPROXY_FEATURES)),)
	GPUPROXY_FEATURE_FLAGS+=--features="$(strip $(GPUPROXY_FEATURES))"
endif

# Add `--cfg unsound_local_offset` flag to allow time crate to get the local time zone. \
See: https://docs.rs/time/0.3.9/time/index.html#feature-flags and \
https://github.com/time-rs/time/issues/293#issuecomment-1005002386
ifeq ($(findstring --cfg unsound_local_offset, $(RUSTFLAGS)), )
RUSTFLAGS+=--cfg unsound_local_offset
endif

all: build

check-all:

test-all:
	cargo test --release --workspace -- --nocapture

fmt:
	cargo fmt --all

mk-dist:
	mkdir -p ./dist/bin

build: mk-dist
	cargo build --release --workspace $(GPUPROXY_FEATURE_FLAGS)
	cp target/release/cluster_c2_plugin ./dist/bin/
	cp target/release/gpuproxy ./dist/bin/
	cp target/release/gpuproxy_worker ./dist/bin/
	cp target/release/migration ./dist/bin/

build-amd: RUSTFLAGS+=-C target-cpu=znver2 -C target-feature=+sse4.1,+sse4.2,+avx,+avx2,+sha,+sse2,+adx
build-amd: CPU_PLATFORM=amd
build-amd: build

build-intel: RUSTFLAGS+=-C target-feature=+sse4.1,+sse4.2
build-intel: CPU_PLATFORM=intel
build-intel: build

up2ftp-amd:
	CPU_PLATFORM=amd ./up2ftp.sh all

up2ftp-intel:
	CPU_PLATFORM=intel ./up2ftp.sh all