#!/bin/sh
export CARGO_HOME='/home/cdy/zhangyi/bot-benchmark/runs/ripgrep-001/toolchain/cargo' RUSTUP_HOME='/home/cdy/zhangyi/bot-benchmark/runs/ripgrep-001/toolchain/rustup' RUSTUP_TOOLCHAIN=1.88.0 CARGO_NET_OFFLINE=false CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR='/tmp/pipeline-baseline-target'
exec '/home/cdy/zhangyi/bot-benchmark/runs/ripgrep-001/toolchain/cargo/bin/cargo' "$@"
