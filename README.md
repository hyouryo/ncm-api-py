# ncm-api-py

从 [ncm-api-rs](https://github.com/SPlayer-Dev/ncm-api-rs) 而来，使用 PyO3 制作成了 Python 包。

只翻译了三个方法
## 安装

```bash
pip install ncm-api-py
```

需要 Python 3.10 或更高版本。


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

    # 播放链接
    url_resp = await client.song_url_v1(186016, level="exhigh")
    item = url_resp.body["data"][0]
    print(item["url"])                            # 无版权/需付费时是 None


asyncio.run(main())
```

## 构建

需要 Rust 工具链和 [maturin](https://github.com/PyO3/maturin)：

```bash
maturin develop --release
```

## 许可证

[MIT](LICENSE)。
