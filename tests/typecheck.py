"""Compile-only examples; mypy --strict also checks expected rejected calls."""

from typing import TYPE_CHECKING

from ncm_api_py import (
    ApiResponse,
    Client,
    JSONValue,
    SearchType,
    SongDetailBody,
    SongSearchBody,
    SongUrlBody,
    create_client,
)


async def valid_calls(search_type: SearchType) -> None:
    client: Client = create_client(cookie=None)
    search: ApiResponse[SongSearchBody] = await client.search("music")
    search_explicit: ApiResponse[SongSearchBody] = await client.search("music", type=1)
    search_none: ApiResponse[SongSearchBody] = await client.search("music", type=None)
    songs = search.body["result"].get("songs", [])
    for song in songs:
        song_id: int = song["id"]
        song_name: str = song["name"]
        print(song_id, song_name)
    print(search_explicit.status, search_none.cookie)
    album_search = await client.search("music", type=10)
    album_json: JSONValue = album_search.body.get("result")
    print(album_json)
    dynamic = await client.search("music", type=search_type)
    print(dynamic.body["code"])
    detail: ApiResponse[SongDetailBody] = await client.song_detail([186016])
    title: str = detail.body["songs"][0]["name"]
    urls: ApiResponse[SongUrlBody] = await client.song_url_v1([186016], level="exhigh")
    url: str | None = urls.body["data"][0]["url"]
    if url is not None:
        print(title, url.upper())
    await client.song_url_v1([186016, 186017], level=None)
    await client.song_url_v1([186016], level="dolby")
    await client.search("music", type=1014)
    await client.search("music", type=1018)
    await client.search("music", limit=None, offset=None)


if TYPE_CHECKING:
    client = Client()
    # --strict enables warn_unused_ignores: each ignore MUST suppress a real error.
    client.song_detail(186016)  # type: ignore[arg-type]
    client.song_detail("186016")  # type: ignore[arg-type]
    client.song_detail(["186016"])  # type: ignore[list-item]
    client.song_detail((186016,))  # type: ignore[arg-type]
    client.song_url_v1([186016], level="unknown")  # type: ignore[arg-type]
    client.search("music", type=2)  # type: ignore[call-overload]
    client.search("music", type="1")  # type: ignore[call-overload]
    client.search("music", limit="30")  # type: ignore[call-overload]
    client.search(123)  # type: ignore[call-overload]
    create_client(cookie=b"cookie")  # type: ignore[arg-type]
    client.not_an_endpoint()  # type: ignore[attr-defined]


async def nullable_url_is_checked() -> None:
    response = await Client().song_url_v1([186016])
    url: str = response.body["data"][0]["url"]  # type: ignore[assignment]
    response.status = 200  # type: ignore[misc]
    print(url)
