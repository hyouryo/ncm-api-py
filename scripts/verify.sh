#!/usr/bin/env bash
# 推送前跑一遍，把 CI 里能本地复现的检查都在本地做掉。
#
# 动机：不要用「提交 + 推送 + 等 CI」当验证手段 —— 一轮好几分钟，而且会往
# commit 历史里塞一堆「再修一下」的提交。
#
# 覆盖范围:
#   1. .github/workflows/*.yml   —— 结构 + shell 语法 + 冒烟判定逻辑（见同目录 py）
#   2. cargo fmt --check
#   3. cargo clippy -D warnings
#   4. ruff check
#   5. maturin build → 装进临时 venv → 真实调用两个接口（3.10+ 的 abi3 wheel）
#
# 跑不了的项目会打印 SKIP 而不是 FAIL（例如没有 Docker，就无法验证 manylinux 交叉编译）。
#
# 用法:
#   bash scripts/verify.sh              # 全部
#   SKIP_WHEEL=1 bash scripts/verify.sh # 跳过耗时的构建+联网测试
#   PYTHON=/path/to/python bash scripts/verify.sh

set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
ROOT="$(pwd)"

PYTHON="${PYTHON:-}"
if [ -z "$PYTHON" ]; then
  for cand in python3 python; do
    if command -v "$cand" >/dev/null 2>&1; then PYTHON="$cand"; break; fi
  done
fi

pass=0
fail=0
skip=0

section() { printf '\n=== %s ===\n' "$1"; }
ok()   { printf '  OK   %s\n' "$1"; pass=$((pass + 1)); }
bad()  { printf '  FAIL %s\n' "$1"; fail=$((fail + 1)); }
miss() { printf '  SKIP %s\n' "$1"; skip=$((skip + 1)); }

# ---------------------------------------------------------------- workflows
section "workflows（结构 / shell 语法 / 冒烟判定）"
if [ -z "$PYTHON" ]; then
  miss "找不到 python，跳过 workflow 检查"
elif "$PYTHON" -c "import yaml" >/dev/null 2>&1; then
  if "$PYTHON" scripts/check_ci_workflows.py; then
    ok "scripts/check_ci_workflows.py"
  else
    bad "scripts/check_ci_workflows.py"
  fi
else
  miss "缺 PyYAML：pip install pyyaml（或用 uvx --with pyyaml python scripts/check_ci_workflows.py）"
fi

# ------------------------------------------------------------------- rust
section "rust（fmt / clippy）"
if command -v cargo >/dev/null 2>&1; then
  if cargo fmt --all --check; then ok "cargo fmt --all --check"; else bad "cargo fmt --all --check"; fi

  # pyo3 的 build script 需要一个解释器：找不到就报 "no Python 3.x interpreter found"。
  # 这里显式把已探测到的 PYTHON 交给它，这样调用方不必自己 export PYO3_PYTHON。
  if [ -n "$PYTHON" ]; then
    pyo3_py="$("$PYTHON" -c 'import sys; print(sys.executable)' 2>/dev/null)"
    if [ -n "$pyo3_py" ]; then
      PYO3_PYTHON="$pyo3_py"
      export PYO3_PYTHON
      echo "      PYO3_PYTHON=$PYO3_PYTHON"
    fi
  fi

  # clippy 需要能构建；离线也能用本地 registry
  if cargo clippy --all-targets --features extension-module -- -D warnings; then
    ok "cargo clippy -D warnings"
  else
    bad "cargo clippy -D warnings"
  fi
else
  miss "找不到 cargo"
fi

# ------------------------------------------------------------------ python
section "python（ruff）"
if command -v ruff >/dev/null 2>&1; then
  if ruff check; then ok "ruff check"; else bad "ruff check"; fi
elif command -v uvx >/dev/null 2>&1; then
  if uvx ruff check; then ok "uvx ruff check"; else bad "uvx ruff check"; fi
else
  miss "找不到 ruff / uvx"
fi

# ------------------------------------------------------------------- wheel
section "wheel（构建 → 安装 → 真实调用）"
if [ "${SKIP_WHEEL:-0}" = "1" ]; then
  miss "SKIP_WHEEL=1"
elif ! command -v maturin >/dev/null 2>&1; then
  miss "找不到 maturin"
elif [ -z "$PYTHON" ]; then
  miss "找不到 python"
else
  out="build/verify-dist"
  rm -rf "$out"
  mkdir -p build
  if maturin build --release --out "$out" >"build/verify-build.log" 2>&1; then
    whl="$(ls "$out"/*.whl 2>/dev/null | head -1)"
    if [ -z "$whl" ]; then
      bad "maturin 没有产出 wheel"
    else
      # 确认是 abi3：一个 wheel 覆盖 3.10+
      case "$whl" in
        *abi3*) ok "产物是 abi3 wheel: $(basename "$whl")" ;;
        *) bad "产物不是 abi3 wheel: $(basename "$whl")（检查 pyproject.toml 的 [tool.maturin] features）" ;;
      esac

      venv="build/verify-venv"
      rm -rf "$venv"
      if "$PYTHON" -m venv "$venv" >/dev/null 2>&1; then
        vpy="$venv/bin/python"
        [ -x "$vpy" ] || vpy="$venv/Scripts/python.exe"
        if "$vpy" -m pip install --quiet --no-index --find-links "$out" ncm-api-py >/dev/null 2>&1; then
          if "$vpy" scripts/smoke_api.py; then
            ok "安装后真实调用 search / song_detail"
          else
            bad "安装后真实调用失败（网络不通会误报，注意看输出）"
          fi
        else
          bad "从 $out 安装 wheel 失败"
        fi
      else
        miss "创建 venv 失败，跳过安装测试"
      fi
    fi
  else
    bad "maturin build 失败，详见 build/verify-build.log"
    tail -20 build/verify-build.log | sed 's/^/       /'
  fi
fi

# ------------------------------------------------------------------ report
printf '\n----------------------------------------\n'
printf '通过 %d  失败 %d  跳过 %d\n' "$pass" "$fail" "$skip"
if [ "$fail" -gt 0 ]; then
  printf '有失败项，先别推。\n'
  exit 1
fi
printf '本地检查全部通过。\n'
