# ncm-api-py

基于 [ncm-api-rs](https://github.com/SPlayer-Dev/ncm-api-rs) 的 PyO3 薄绑定，
目前只提供 `search`、`song_detail`、`song_url_v1` 三个异步接口。

绑定层只转换参数、响应和异常，不添加业务校验、重试、备用接口、音质降级或失败默认结果。
上游库本身的默认值和行为保持不变。

## 安装与用法

```bash
pip install ncm-api-py
```

需要 CPython 3.10 或更高版本。在运行中的 asyncio 事件循环内调用接口：

```python
import asyncio

import ncm_api_py


async def main() -> None:
    client = ncm_api_py.create_client()  # 或 Client(cookie="...")
    search = await client.search("晴天 周杰伦", type=1, limit=3)
    print(search.body)

    detail = await client.song_detail([186016, 186017])
    print(detail.body)

    urls = await client.song_url_v1([186016], level="exhigh")
    print(urls.body)


asyncio.run(main())
```

## 转换规则

- `search(keywords, type=None, limit=None, offset=None)`：整数使用有符号 64 位。
  `None` 表示不传对应查询参数，由上游决定默认值（当前为 `1`、`30`、`0`）。
  不检查搜索类型、空关键词、limit 是否为正或 offset 是否非负。
- `song_detail(ids)` / `song_url_v1(ids, level=None)`：ID 序列转换成逗号分隔字符串。
  使用 PyO3 对 `Vec<i64>` 的标准转换，不额外限制只能传 list，不排除 bool，
  不检查 ID 是否为正、是否重复或序列是否为空。整数仍必须能转换为 i64。
- `level=None` 表示不传音质参数，由上游使用默认值（当前为 `standard`）；
  其他字符串原样传递，不做白名单检查或降级。
- 返回 `Awaitable[ApiResponse]`；响应有只读属性 `status`、`body`、`cookie`。
  `body` 通过 `pythonize` 转成原生 Python 值，不强制为字典，不补字段，
  保留 JSON null 为 `None`。播放链接不可用时不会替换为其他链接。
- 上游 `NcmError` 转为 `RuntimeError`，保留错误文本；参数类型与溢出错误由
  PyO3 自身抛出，不捕获后返回默认结果。
- 不自动保存响应 cookie，也不额外发起请求。

## 自动生成类型声明

`ncm_api_py.pyi` 由 `pyo3-stub-gen` 从 Rust 定义生成，不手写维护。
异步返回类型在 Rust 宏属性中声明为 `Awaitable[ApiResponse]`，响应 `body` 为
`Any`，不为第三方响应虚构固定结构。

修改公开 API 后运行并提交生成结果：

```bash
cargo run --locked --bin stub_gen
```

生成脚本针对生成器 0.23.2 的 raw identifier 问题，在生成前规范化参数名元数据
（如 `r#type` → `type`），不替换生成文件中的文档或默认字符串。

## 开发依赖与检查

需要 Rust 工具链、Python 和 [maturin](https://github.com/PyO3/maturin)。
Windows 还需要 Visual Studio C++ Build Tools 和 Windows SDK。
运行 Rust 测试时，需要把 Python 安装目录（包含 `python3xx.dll`）加入 `PATH`，
否则测试程序会报 `STATUS_DLL_NOT_FOUND`；仅激活虚拟环境可能不够。
Rust 依赖由 Cargo 安装：`ncm-api-rs`、`pyo3`、`pyo3-async-runtimes`、
`pythonize`、`serde_json` 和 `pyo3-stub-gen`；不需要新增 Python 运行时库。
`ruff` 和 `mypy` 仅用于开发检查。

在虚拟环境中执行：

```bash
pip install maturin ruff mypy
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
cargo run --locked --bin stub_gen
maturin develop
ruff check .
ruff format --check .
mypy --strict checks/typecheck.py
python -m unittest discover -s checks -v
```

Rust 测试不启用 `extension-module`，以便链接 Python。maturin 构建使用
配置中的 `extension-module` 和 `abi3`，生成 cp310-abi3 wheel。
测试只检查转换和接口，不连接网易云、不需要 cookie。

## 许可证

[MIT](LICENSE)。
