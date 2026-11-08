#!/usr/bin/env bash
# 构建并运行 gui/(GPUI 桌面壳,无数据库域;数据库后端 = vendor/dbx)。
# 用法:script/linop-gui.sh [start|stop|status|logs|build]
set -euo pipefail
cd "$(dirname "$0")/.."

NAME=linop-gui
IMAGE=linop-build:1
REPO=/home/xiuran/Linop

case "${1:-start}" in
  build)
    # gui workspace 依赖根仓库(crates/linop-dbx 路径依赖 + vendor/dbx patch),
    # 需挂载整个仓库而非仅 gui/。
    docker run --rm -v "$REPO":/home/xiuran/Linop \
      -v /home/xiuran/.cargo/registry:/usr/local/cargo/registry \
      -v /home/xiuran/.cargo/git:/usr/local/cargo/git \
      "$IMAGE" bash -c "cd /home/xiuran/Linop/gui && cargo build -p linop --release"
    ;;
  start)
    docker rm -f "$NAME" 2>/dev/null || true
    docker run -d --name "$NAME" \
      -v "$REPO":/home/xiuran/Linop \
      -v /tmp/.X11-unix:/tmp/.X11-unix \
      -v /mnt/wslg:/mnt/wslg \
      --device /dev/dri \
      -e DISPLAY=:0 \
      "$IMAGE" bash -c "exec /home/xiuran/Linop/gui/target/release/linop"
    echo "Linop GUI 已启动(Windows 桌面应出现 Linop 窗口)"
    ;;
  stop)
    docker rm -f "$NAME"
    ;;
  status)
    docker ps -a --filter "name=$NAME" --format '{{.Names}}: {{.Status}}'
    docker exec "$NAME" bash -c "DISPLAY=:0 xwininfo -root -tree 2>/dev/null | grep -iE 'linop' | head -2" 2>/dev/null || true
    ;;
  logs)
    docker logs -f "$NAME"
    ;;
  *)
    echo "用法:$0 [start|stop|status|logs|build]"; exit 1 ;;
esac
