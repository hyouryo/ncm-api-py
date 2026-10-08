"""装好 wheel 之后跑一遍真实调用，确认三个接口在本机真的能用。

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

    # 播放链接：单 id、显式音质、多 id 三种形态过一遍。
    # 未登录时 url 基本是 None（版权限制），而**状态码仍是 200**，
    # 所以断言的是「接口通且返回了 data 结构」，不是「一定有链接」。
    for label, kwargs in (("默认", {}), ("exhigh", {"level": "exhigh"})):
        u: ApiResponse = await client.song_url_v1(first, **kwargs)
        assert u.status == 200, f"song_url_v1({label}) 失败: status={u.status}"
        data = u.body["data"]
        assert data, f"song_url_v1({label}) 返回空 data"
        assert data[0].get("id") == first, f"song_url_v1({label}) 返回的 id 不对"
        print(f"song_url_v1({label:6}) level={data[0].get('level')} url={'有' if data[0]['url'] else '无'}")

    # 多 id：绑定层拼成 "1,2" 交给 Rust；实测返回条数与顺序都正确，
    # 但这是服务端对 "[1,2]" 的宽容解析，属未文档化行为。
    multi = [186016, 186017]
    m: ApiResponse = await client.song_url_v1(multi, level="standard")
    assert m.status == 200, f"song_url_v1(多 id) 失败: status={m.status}"
    got = [x.get("id") for x in m.body["data"]]
    assert got == multi, f"多 id 返回 {got}，与请求 {multi} 不一致"
    print(f"song_url_v1(多 id ) 返回 {got}")

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
