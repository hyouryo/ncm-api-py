"""装好 wheel 之后跑一遍真实调用，确认两个接口在本机真的能用。

这是唯一必须联网的本地检查：用的是线上接口，所以网络不通会失败 —— 那不是代码问题。
其余检查（workflow / fmt / clippy / ruff / 构建）全部离线可跑。
"""

from __future__ import annotations

import asyncio
import pathlib
import sys

import ncm_api_py
from ncm_api_py import ApiResponse, Client, create_client

pkg = pathlib.Path(ncm_api_py.__file__).parent
pyd = [p.name for p in pkg.iterdir() if p.suffix in (".pyd", ".so")]
print(f"python {sys.version.split()[0]} | 扩展 {pyd}")
print(f"类型标注: py.typed={(pkg / 'py.typed').exists()} __init__.pyi={(pkg / '__init__.pyi').exists()}")

assert (pkg / "py.typed").exists(), "wheel 里缺 py.typed"
assert (pkg / "__init__.pyi").exists(), "wheel 里缺 __init__.pyi"


async def main() -> None:
    client: Client = create_client()

    r: ApiResponse = await client.search("晴天 周杰伦", type=1, limit=3)
    songs = r.body["result"]["songs"]
    assert r.status == 200 and songs, f"search 失败: status={r.status}"
    print(f"search        status={r.status} songs={len(songs)}")

    first = songs[0]["id"]
    for label, ids in (("int", first), ("str", str(first)), ("list", [186016, 186017]),
                       ("csv", "186016, 186017")):
        d: ApiResponse = await client.song_detail(ids)
        assert d.status == 200, f"song_detail({label}) 失败: status={d.status}"
        print(f"song_detail({label:4}) ids={[s['id'] for s in d.body['songs']]}")

    # 未登录也应拿到 cookie
    assert r.cookie, "响应里没有 cookie"

    # 类型错误要抛出 TypeError 而不是静默
    try:
        await client.song_detail({"a": 1})
    except TypeError:
        pass
    else:
        raise AssertionError("dict 应该被拒绝")


asyncio.run(main())
print("smoke_api 通过")
