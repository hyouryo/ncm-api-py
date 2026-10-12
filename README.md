# ncm-api-py

基于 [ncm-api-rs](https://github.com/SPlayer-Dev/ncm-api-rs) 的 PyO3 绑定，
只提供 `search`、`song_detail`、`song_url_v1` 三个异步接口。

## 安装

```bash
pip install ncm-api-py
```

需要 CPython 3.10 或更高版本。

## 用法

三个方法返回可 `await` 的对象；在运行中的 asyncio 事件循环内调用。
结果是 `ApiResponse`，具有只读属性 `status`、`body`、`cookie`。
`body` 保留上游原始 JSON 数据，不转换成额外的数据模型。

```python
import asyncio

import ncm_api_py


async def main() -> None:
    client = ncm_api_py.create_client()  # 或 Client(cookie="...")

    response = await client.search("晴天 周杰伦", type=1, limit=3)
    for song in response.body["result"].get("songs", []):
        print(song["id"], song["name"])

    detail = await client.song_detail([186016, 186017])
    for song in detail.body["songs"]:
        print(song["name"])

    urls = await client.song_url_v1([186016], level="exhigh")
    for item in urls.body["data"]:
        url = item["url"]
        if url is not None:
            print(url)
        else:
            print("无可用播放链接")


asyncio.run(main())
```

## 参数与类型

- `ids`：必须是非空 `list[int]`，每个 ID 在 `1..2**63-1` 范围内。
  不接受单个整数、逗号字符串、tuple、bool、浮点数或仅实现 `__index__` 的对象。
- `search`：`type` 默认 `1`，`limit` 默认 `30` 且必须大于零，
  `offset` 默认 `0` 且不得为负。三个整数参数均不接受 bool，使用有符号 64 位整数范围。
  搜索类型：`1` 单曲、`10` 专辑、`100` 歌手、`1000` 歌单、`1002` 用户、
  `1004` MV、`1006` 歌词、`1009` 电台、`1014` 视频、`1018` 综合、`2000` 声音。
- `song_url_v1`：`level` 默认 `standard`，支持 `standard`、`higher`、`exhigh`、
  `lossless`、`hires`、`jyeffect`、`sky`、`dolby`、`jymaster`。
- 所有可选参数继续接受 `None`，表示使用默认值。
- 错误参数在发起请求前抛出 `TypeError`、`ValueError` 或 `OverflowError`。
  上游请求失败仍抛出 `RuntimeError`，保留上游错误文本。
- 成功响应不保证有播放权限；`url` 明确为 `str | None`，必须判空。

随包提供类型 stub：`ApiResponse[BodyT]`、搜索类型/音质 `Literal`，
以及 `SongSearchBody`、`SongDetailBody`、`SongUrlBody` 等 `TypedDict`。
默认/单曲搜索返回歌曲结构；其他搜索类型返回递归 JSON 类型，避免把专辑、
用户等结果错误标成歌曲。可变搜索类型返回两类响应的联合类型。
这些类型仅用于静态检查，不验证上游响应；只描述常用字段，实际响应可能有更多字段。
Python 类型系统把 bool 视为 int 子类，静态检查无法完全排除 bool，运行时会严格拒绝。
类型别名在 stub 中使用，运行时注解建议使用 `from __future__ import annotations`
及 `if TYPE_CHECKING` 导入。

## 构建与检查

需要 Rust 工具链、Python 和 [maturin](https://github.com/PyO3/maturin)。
在虚拟环境中构建：

```bash
maturin develop --release
```

Windows 需要 Visual Studio C++ Build Tools 和 Windows SDK，推荐使用 Developer
PowerShell/Command Prompt，确保 `PATH`、`INCLUDE`、`LIB` 配置完整。
不要设置全局 `CRATE_CC_NO_DEFAULTS=1`：这会禁用 `cc` 的平台默认编译标志，
包括 Linux 共享库所需的 PIC、交叉编译目标及 sysroot 等。
CI 使用原生 ARM runner 构建 ARM Linux wheel，并在匹配 glibc/musl 的环境中安装测试。

```bash
pip install ruff mypy
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
ruff check .
ruff format --check .
mypy --strict tests/typecheck.py
python -m unittest discover -s tests -v
```

Rust 测试需要能够链接 Python，故不启用 `extension-module`；maturin 构建
wheel 时自动使用配置中的 `extension-module` 和 `abi3`，生成 cp310-abi3 wheel。
Python 测试只验证接口、参数和边界条件，不连接网易云、不需要 cookie。

## 许可证

[MIT](LICENSE)。
