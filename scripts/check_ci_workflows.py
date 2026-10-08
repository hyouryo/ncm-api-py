"""离线校验 .github/workflows/*.yml。

存在的理由：CI 里那几段 shell / 条件判断光靠眼睛看很容易错，而跑一次 CI 要等好几分钟
并且会把 commit 历史弄脏（每个字母的改动一次提交）。这个脚本把「能本地测的」都测掉：

1. YAML 能否解析、job / step 结构是否符合预期；
2. 抽出的 shell 片段能否通过 `bash -n` 语法检查；
3. 抽出的「冒烟测试」平台判定逻辑，用模拟的 uname(OS/架构) + 假 python 跑一遍，
   确认哪些 wheel 会被真正安装、哪些会被正确跳过；
4. 两个 job 里复制粘贴的同一段脚本是否仍然完全一致（防止改一处漏一处）。

只用标准库 + PyYAML；不依赖联网、Docker 或真实 wheel。

用法: python scripts/check_ci_workflows.py
"""

from __future__ import annotations

import os
import pathlib
import shutil
import subprocess
import sys
import tempfile

try:
    import yaml
except ImportError:  # pragma: no cover
    sys.exit("需要 PyYAML：pip install pyyaml（或 uvx --with pyyaml python ...）")

ROOT = pathlib.Path(__file__).resolve().parent.parent
WORKFLOWS = ROOT / ".github" / "workflows"

# 每个 job 期望存在的 PyO3/maturin-action 步骤，以及它的 manylinux 取值
EXPECTED_MATURIN = {
    "Release.yml": {
        "linux": "manylinux_2_28",
        "musllinux": "musllinux_1_2",
        "windows": None,
        "macos": None,
        "sdist": None,
        "release": None,
    },
    "CI.yml": {"build-linux": "manylinux_2_28", "build-native": None},
}

# 模拟宿主环境 × wheel 平台标签 -> 是否应该在冒烟测试里真正安装
WHEELS = {
    "linux-x86_64": "manylinux_2_28_x86_64",
    "linux-aarch64": "manylinux_2_28_aarch64",
    "musllinux-x86_64": "musllinux_1_2_x86_64",
    "macos-arm64": "macosx_11_0_arm64",
    "macos-x86_64": "macosx_10_12_x86_64",
    "windows-amd64": "win_amd64",
    "windows-arm64": "win_arm64",
}
SMOKE_CASES = [
    ("Linux", "x86_64", "linux-x86_64", True),
    ("Linux", "x86_64", "musllinux-x86_64", True),
    ("Linux", "aarch64", "linux-aarch64", True),
    ("Linux", "x86_64", "linux-aarch64", False),  # 交叉编译：本次 CI 真踩过的坑
    ("Linux", "x86_64", "windows-amd64", False),  # 跨 OS
    ("Darwin", "arm64", "macos-arm64", True),
    ("Darwin", "arm64", "macos-x86_64", False),  # macOS x86_64 属交叉
    ("Darwin", "x86_64", "macos-x86_64", True),
    ("MINGW64_NT-10.0", "x86_64", "windows-amd64", True),
    ("MINGW64_NT-10.0", "x86_64", "windows-arm64", False),
    ("MINGW64_NT-10.0", "x86_64", "linux-x86_64", False),
]

failures: list[str] = []


def find_bash() -> str | None:
    if os.name == "nt":
        for cand in (
            r"C:\Program Files\Git\bin\bash.exe",
            r"C:\Program Files (x86)\Git\bin\bash.exe",
        ):
            if pathlib.Path(cand).exists():
                return cand
    return shutil.which("bash")


def to_posix(p: pathlib.Path, bash_is_windows: bool) -> str:
    s = p.resolve().as_posix()
    if bash_is_windows and len(s) > 2 and s[1] == ":":
        return "/" + s[0].lower() + s[2:]
    return s


def check(name: str, ok: bool, detail: str = "") -> None:
    print(f"{'OK  ' if ok else 'FAIL'} {name}" + (f"  {detail}" if detail else ""))
    if not ok:
        failures.append(name)


def load(name: str) -> dict:
    return yaml.safe_load((WORKFLOWS / name).read_text(encoding="utf-8"))


def steps_of(doc: dict, job: str) -> list[dict]:
    return doc["jobs"][job].get("steps", [])


def maturin_steps(doc: dict) -> dict[str, dict]:
    out = {}
    for job, spec in doc["jobs"].items():
        for st in spec.get("steps", []):
            if str(st.get("uses", "")).startswith("PyO3/maturin-action"):
                out.setdefault(job, st)
    return out


def main() -> int:
    print(f"工作流目录: {WORKFLOWS}\n")

    # ---- 1. YAML 解析 + manylinux 取值 ----
    docs = {}
    for name in ("CI.yml", "Release.yml"):
        docs[name] = load(name)
        check(f"{name} 可解析", True, f"jobs={list(docs[name]['jobs'])}")

    for name, expected in EXPECTED_MATURIN.items():
        found = maturin_steps(docs[name])
        check(
            f"{name} maturin 步骤集合",
            set(found) == set(expected),
            f"实际={sorted(found)}",
        )
        for job, want_ml in expected.items():
            if job not in found:
                continue
            got = found[job].get("with", {}).get("manylinux")
            check(
                f"{name}:{job} manylinux",
                got == want_ml,
                f"期望={want_ml!r} 实际={got!r}",
            )
            args = found[job].get("with", {}).get("args", "")
            check(f"{name}:{job} 不再用 --find-interpreter（abi3 不需要）", "--find-interpreter" not in args)

    # ---- 2. 所有 run 块语法检查 ----
    bash = find_bash()
    if not bash:
        print("跳过 shell 语法检查：找不到 bash")
    else:
        is_win = os.name == "nt"
        n = 0
        for name, doc in docs.items():
            for job, spec in doc["jobs"].items():
                for i, st in enumerate(spec.get("steps", [])):
                    run = st.get("run")
                    if not run:
                        continue
                    n += 1
                    with tempfile.NamedTemporaryFile(
                        "w", suffix=".sh", delete=False, encoding="utf-8", newline="\n"
                    ) as fh:
                        fh.write(run)
                        tmp = pathlib.Path(fh.name)
                    proc = subprocess.run(
                        [bash, "-n", to_posix(tmp, is_win)], capture_output=True, text=True
                    )
                    check(
                        f"{name}:{job} step[{i}] shell 语法",
                        proc.returncode == 0,
                        proc.stderr.strip()[:120],
                    )
                    tmp.unlink()
        print(f"      （共检查 {n} 个 run 块）")

    # ---- 3. 冒烟测试逻辑：复制体一致性 + 行为 ----
    smokes: dict[str, str] = {}
    for job, spec in docs["CI.yml"]["jobs"].items():
        for st in spec.get("steps", []):
            if st.get("name") == "Smoke test the built wheel":
                smokes[job] = st["run"]
    check("CI.yml 两个 build job 都有冒烟测试", set(smokes) == {"build-linux", "build-native"})
    if len(smokes) > 1:
        vals = list(smokes.values())
        check("两处冒烟脚本完全一致（防复制体漂移）", len(set(vals)) == 1)

    if bash and smokes:
        script = next(iter(smokes.values()))
        is_win = os.name == "nt"
        with tempfile.TemporaryDirectory() as tmp:
            tmpdir = pathlib.Path(tmp)
            for idx, (sysname, arch, key, want) in enumerate(SMOKE_CASES):
                case = tmpdir / f"{idx:02d}-{sysname}-{arch}-{key}"
                (case / "dist").mkdir(parents=True)
                (case / "dist" / f"ncm_api_py-0.1.0-cp310-abi3-{WHEELS[key]}.whl").write_text(
                    "", encoding="utf-8"
                )

                stub_bin = case / "stubbin"
                stub_bin.mkdir()
                calls = case / "calls.log"
                py = stub_bin / "python"
                py.write_text(
                    '#!/usr/bin/env bash\necho "python $*" >> "'
                    + to_posix(calls, is_win)
                    + '"\n',
                    encoding="utf-8",
                    newline="\n",
                )
                py.chmod(0o755)

                runner = case / "runner.sh"
                runner.write_text(
                    "#!/usr/bin/env bash\n"
                    "uname() {\n"
                    '  case "$1" in\n'
                    f"    -s) echo '{sysname}' ;;\n"
                    f"    -m) echo '{arch}' ;;\n"
                    "    *) echo unknown ;;\n"
                    "  esac\n"
                    "}\n"
                    'export PATH="' + to_posix(stub_bin, is_win) + ':$PATH"\n' + script + "\n",
                    encoding="utf-8",
                    newline="\n",
                )
                proc = subprocess.run(
                    [bash, to_posix(runner, is_win)],
                    cwd=case,
                    capture_output=True,
                    text=True,
                    encoding="utf-8",
                    errors="replace",
                    env=dict(os.environ),
                )
                installed = calls.exists() and "pip install" in calls.read_text(encoding="utf-8")
                check(
                    f"冒烟判定 {sysname}/{arch} + {key}",
                    installed == want and proc.returncode == 0,
                    f"期望装={want} 实际装={installed} exit={proc.returncode}",
                )

    # ---- 4. 不可见字符扫描 ----
    # 中文标点在 UTF-8 下完全合法（注释、echo、YAML 字符串里都有），不要误报。
    # 真正会咬人的是 BOM / 零宽 / 不换行空格，以及 CRLF（Linux 上 ruff、shell 步骤会受影响）。
    invisible = {
        "\ufeff": "BOM",
        "\u200b": "零宽空格",
        "\u200c": "零宽非连接符",
        "\u200d": "零宽连接符",
        "\u00a0": "不换行空格",
        "\u2028": "行分隔符",
        "\u2029": "段分隔符",
    }
    for name in ("CI.yml", "Release.yml"):
        raw = (WORKFLOWS / name).read_bytes()
        text = raw.decode("utf-8")
        hits = [
            f"L{i + 1}:{invisible[ch]}"
            for i, line in enumerate(text.splitlines())
            for ch in line
            if ch in invisible
        ]
        check(f"{name} 无不可见字符", not hits, ",".join(hits[:5]))
        check(f"{name} 用 LF 换行", b"\r\n" not in raw, f"CR 字节数={raw.count(bytes([13]))}")
        check(f"{name} 无 UTF-8 BOM", not raw.startswith(b"\xef\xbb\xbf"))

    print()
    if failures:
        print(f"失败 {len(failures)} 项:")
        for f in failures:
            print("  -", f)
        return 1
    print("全部通过")
    return 0


if __name__ == "__main__":
    sys.exit(main())
