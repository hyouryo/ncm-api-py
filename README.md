# ncm-api-py

从 [ncm-api-rs](https://github.com/SPlayer-Dev/ncm-api-rs) 而来，使用 PyO3 制作成了 Python 包。

目前只翻译了两个接口，且都是原样转发：

| Python | 对应 Rust |
| --- | --- |
| `Client.search(keywords, type=None, limit=None, offset=None)` | `ApiClient::search`（`/search`） |
| `Client.song_detail(ids)` | `ApiClient::song_detail`（`/song/detail`） |

不增删任何请求参数，也不改动字段名、状态码与错误语义。

## 安装

```bash
pip install ncm-api-py
```

需要 Python 3.10 或更高版本。

## 用法

两个方法都是 `async`，返回 `ApiResponse`（含 `status` / `body` / `cookie`）：

```python
import asyncio

import ncm_api_py


async def main() -> None:
    client = ncm_api_py.create_client()          # 也可以传 cookie

    # 歌曲搜索
    resp = await client.search("晴天 周杰伦", type=1, limit=3)
    print(resp.status, resp.body["result"]["songs"])

    # 通过 id 获取歌曲信息
    detail = await client.song_detail(186016)
    print(detail.body["songs"][0]["name"])

    # ids 也接受列表或逗号字符串
    await client.song_detail([186016, 186017])
    await client.song_detail("186016,186017")


asyncio.run(main())
```

`search` 未显式传入的 `type` / `limit` / `offset` 不会写进 `Query`，由 `ncm-api-rs`
自己套用默认值（`1` / `30` / `0`）。接口返回非 200 时抛出 `RuntimeError`，
文案取自 `ncm_api_rs::NcmError`。

## 构建

需要 Rust 工具链和 [maturin](https://github.com/PyO3/maturin)：

```bash
maturin develop --release
```

## 许可证

[MIT](LICENSE)。
