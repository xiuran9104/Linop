#!/usr/bin/env bash
# 重建 mesa dzn(Dozen:D3D12 上的 Vulkan)驱动 —— 已在本仓实测验证的完整流程。
#
# 背景:发行版(Debian/Ubuntu/Fedora)的 mesa 包均不含 dzn,需自编;
# 且 mesa 版本必须与 WSL 驱动的 D3D12 SDK 版本对齐(本机 WSL 驱动
# 610.57 → SDK 610 → mesa 23.3.6 + DirectX-Headers 1.610.0)。
#
# 前提:WSL 驱动暴露 D3D12 适配器(用 script/dxg-probe.c 检查;若
# GRAPHICS/COMPUTE 计数均为 0,说明宿主只启用了 CUDA 直通,dzn 无从
# 起作用 —— 这是本机实测的结论,见 README「渲染后端」一节)。
#
# 用法:在任意 Linux 容器/主机上执行(需要 root 与构建工具):
#   script/build-mesa-dzn.sh [输出目录]   # 默认 ./build-dzn-out

set -euo pipefail
OUT="${1:-build-dzn-out}"

echo "== 1/5 安装构建依赖"
apt-get update -qq
apt-get install -y --no-install-recommends -qq \
  meson ninja-build pkg-config python3-mako python3-yaml python3-ply \
  flex bison libdrm-dev zlib1g-dev libzstd-dev libexpat1-dev \
  directx-headers-dev curl ca-certificates g++ > /dev/null

echo "== 2/5 DirectX-Headers 1.610.0(对齐 WSL 驱动 SDK 610)"
cd "$(mktemp -d)"
curl -sL -o dxh.tar.gz https://codeload.github.com/microsoft/DirectX-Headers/tar.gz/refs/tags/v1.610.0
tar xzf dxh.tar.gz
cd DirectX-Headers-1.610.0
meson setup build -Dbuild-test=false --prefix /usr/local/dx610 > /dev/null
ninja -C build > /dev/null
meson install -C build > /dev/null

echo "== 3/5 mesa 23.3.6 源码"
cd ..
curl -sL -o mesa.tar.xz https://archive.mesa3d.org/mesa-23.3.6.tar.xz
tar xJf mesa.tar.xz
cd mesa-23.3.6

echo "== 4/5 配置并编译(仅 dzn,产物最小;约 3-5 分钟)"
PKG_CONFIG_PATH=/usr/local/dx610/lib/x86_64-linux-gnu/pkgconfig:/usr/local/dx610/lib/pkgconfig \
  meson setup build-dzn \
    -Dvulkan-drivers=microsoft-experimental \
    -Dgallium-drivers= -Dplatforms=[] \
    -Dglx=disabled -Degl=disabled -Dgbm=disabled -Dllvm=disabled \
    -Dgles1=disabled -Dgles2=disabled -Dopengl=false > /dev/null
ninja -C build-dzn -j"$(nproc)" > /dev/null

echo "== 5/5 落位到 $OUT"
mkdir -p "$OUT"
cp build-dzn/src/microsoft/vulkan/libvulkan_dzn.so "$OUT/"
cat > "$OUT/dzn_icd.json" <<'EOF'
{
  "file_format_version": "1.0.0",
  "ICD": {"library_path": "/usr/local/vulkan-dzn/libvulkan_dzn.so", "api_version": "1.3.0"}
}
EOF

cat <<USAGE

完成。使用方法(容器内):
  docker run ... \\
    -v $PWD/$OUT:/usr/local/vulkan-dzn:ro \\
    --device /dev/dxg -v /usr/lib/wsl/lib:/usr/lib/wsl/lib:ro \\
    -e LD_LIBRARY_PATH=/usr/lib/wsl/lib \\
    -e VK_DRIVER_FILES=/usr/local/vulkan-dzn/dzn_icd.json ...命令
验证:vulkaninfo --summary 应列出 D3D12 适配器(而非 llvmpipe)。
USAGE
