# ncm-api-py

从 [ncm-api-rs](https://github.com/SPlayer-Dev/ncm-api-rs) 而来，使用 PyO3 制作成了 Python 包。

原样转发下面三个接口，不增删请求参数，也不改动字段名、状态码与错误语义：

| Python | 对应 Rust | 上游路径 |
| --- | --- | --- |
| `Client.search(keywords, type=None, limit=None, offset=None)` | `ApiClient::search` | `/api/search/get`（`type=2000` 时走 `/api/search/voice/get`） |
| `Client.song_detail(ids)` | `ApiClient::song_detail` | `/api/v3/song/detail` |
| `Client.song_url_v1(ids, level=None)` | `ApiClient::song_url_v1` | `/api/song/enhance/player/url/v1` |

## 安装

```bash
pip install ncm-api-py
```

需要 Python 3.10 或更高版本。

扩展基于 CPython stable ABI（`abi3`）构建，wheel 只带 `cp310-abi3` 标签，
**同一个 wheel 在 3.10 到 3.14 上都能装**，不需要按版本单独发行。
代价是只能用 3.10 就已存在的 CPython C API，并且不支持 free-threaded 版本（3.13t/3.14t）。

## 用法

三个方法都是 `async`，返回 `ApiResponse`（含 `status` / `body` / `cookie`）：

```python
import asyncio

import ncm_api_py


async def main() -> None:
    client = ncm_api_py.create_client()          # 也可以传 cookie

    # 歌曲搜索
    resp = await client.search("晴天 周杰伦", type=1, limit=3)
    for song in resp.body["result"]["songs"]:
        print(song["id"], song["name"])

    # 通过 id 获取歌曲信息；也接受列表或逗号字符串
    detail = await client.song_detail(186016)
    print(detail.body["songs"][0]["name"])
    await client.song_detail([186016, 186017])
    await client.song_detail("186016,186017")

    # 播放链接
    url_resp = await client.song_url_v1(186016, level="exhigh")
    item = url_resp.body["data"][0]
    print(item["url"])                            # 无版权/需付费时是 None


asyncio.run(main())
```

`ids` 统一接受这几种形态：`186016` / `"186016"` / `[186016, 186017]` / `(186016, 186017)` /
`"186016,186017"`。字符串按 `,` 切分、去空白、丢掉空串，这一步由 `ncm-api-rs` 完成。

## 需要注意的行为

都是实测出来的，不是照文档抄的：

- **未显式传入的参数不会写进 `Query`**，由 `ncm-api-rs` 自己套默认值
  （`type`→`1`、`limit`→`30`、`offset`→`0`、`level`→`standard`）。
- **`song_url_v1` 的状态码是 200 也不代表一定拿得到链接。** 无版权或需要付费时
  `body["data"][i]["url"]` 是 `None`，必须自己判空。
- **`song_url_v1` 返回的 `level` 字段不可信。** 实测它有三种表现：拿不到链接时是 `None`；
  请求的音质真的生效时跟随请求值；没生效时退回 `"standard"`。判断音质请用 `br`（码率，
  拿不到时是 `0`），不要用 `level`。
- **播放链接是每次请求重新签名的临时 URL**，不要缓存、也不要跨请求比较。
- **`song_url_v1` 的多 id 属未文档化行为。** `ncm-api-rs` 只取 `Query` 的 `id`
  且不做切分，直接拼进 `format!("[{}]", id)`；单个 id 时是合法 JSON 数组，传多个
  （拼成 `"1,2"`）依赖上游对 `[1,2]` 的宽容解析 —— 实测可用，但建议只传单个 id。
- **`search` 的返回结构随 `type` 变化**：普通搜索在 `body["result"]["songs"]`，
  `type=2000`（声音）在 `body["data"]["resources"]`。
- 上游返回非 200 时抛出 `RuntimeError`，文案取自 `ncm_api_rs::NcmError`。

## 构建

需要 Rust 工具链和 [maturin](https://github.com/PyO3/maturin)：

```bash
maturin develop --release
```

## 许可证

[MIT](LICENSE)。
