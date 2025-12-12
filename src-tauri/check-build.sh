#!/bin/bash

# 编译测试脚本
# 用于在资源受限的环境中测试代码

echo "======================================"
echo "RW CDC SR - 编译测试"
echo "======================================"
echo ""

# 检查 Rust 版本
echo "Rust 版本："
rustc --version
cargo --version
echo ""

# 清理旧的构建产物
echo "清理构建产物..."
cargo clean
echo ""

# 只检查语法，不进行链接
echo "检查代码语法..."
cargo check --lib --message-format=short 2>&1 | head -100

echo ""
echo "======================================"
echo "如果遇到 SIGKILL 错误，这通常是由于："
echo "1. 系统内存不足"
echo "2. 编译过程占用资源过多"
echo ""
echo "建议："
echo "- 在本地环境运行完整编译"
echo "- 或使用 'cargo build --release' 进行优化编译"
echo "======================================"
