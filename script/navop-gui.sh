#!/usr/bin/env bash
# 在 WSLg 显示上启动/停止 vendored navop GUI(Windows 桌面直接可见)。
# 用法:script/navop-gui.sh [start|stop|status|logs]
set -euo pipefail
cd "$(dirname "$0")/.."

NAME=navop-gui
IMAGE=navop-build:2

case "${1:-start}" in
  start)
    docker rm -f "$NAME" 2>/dev/null || true
    docker run -d --name "$NAME" \
      -v "$PWD/vendor/navop:/w" \
      -v /tmp/.X11-unix:/tmp/.X11-unix \
      -v /mnt/wslg:/mnt/wslg \
      --device /dev/dri \
      -e DISPLAY=:0 \
      "$IMAGE" bash -c "exec /w/target/debug/navop"
    echo "navop GUI 已启动(Windows 桌面应出现 Navop 窗口;docker logs $NAME 看日志)"
    ;;
  stop)
    docker rm -f "$NAME"
    ;;
  status)
    docker ps -a --filter "name=$NAME" --format '{{.Names}}: {{.Status}}'
    docker exec "$NAME" bash -c "DISPLAY=:0 xwininfo -root -tree 2>/dev/null | grep -i navop | head -2" 2>/dev/null || true
    ;;
  logs)
    docker logs -f "$NAME"
    ;;
  *)
    echo "用法:$0 [start|stop|status|logs]"; exit 1 ;;
esac
