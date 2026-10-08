"""ncm-api-py 的类型标注（Type stubs）.

对 ``ncm-api-rs`` 的 PyO3 绑定，目前只包含两个接口：

- :meth:`Client.search` —— 歌曲搜索
- :meth:`Client.song_detail` —— 通过 id 获取歌曲信息

两者都是 ``async`` 方法，成功时返回 :class:`ApiResponse`。
"""

from collections.abc import Awaitable, Iterable
from typing import Any

__all__ = ["ApiResponse", "Client", "create_client"]


class ApiResponse:
    """对应 ``ncm_api_rs::ApiResponse``.

    状态码非 200 时不会返回该对象，而是抛出 :class:`RuntimeError`
    （文案取自 ``ncm_api_rs::NcmError``）。
    """

    @property
    def status(self) -> int:
        """响应状态码，对应 ``ApiResponse::status``."""

    @property
    def body(self) -> dict[str, Any] | list[Any] | str | int | float | bool | None:
        """响应体，对应 ``ApiResponse::body``（JSON 已映射为 Python 对象）."""

    @property
    def cookie(self) -> list[str]:
        """响应 Set-Cookie，对应 ``ApiResponse::cookie``."""


class Client:
    """对应 ``ncm_api_rs::ApiClient``."""

    def __init__(self, cookie: str | None = None) -> None:
        """创建一个新的 API 客户端，对应 ``ApiClient::new``."""

    def search(
        self,
        keywords: str,
        type: int | None = None,
        limit: int | None = None,
        offset: int | None = None,
    ) -> Awaitable[ApiResponse]:
        """歌曲搜索，对应 ``ApiClient::search``（``/search``）.

        :param keywords: 搜索关键字，对应 ``Query`` 的 ``keywords``
        :param type: 搜索类型，对应 ``Query`` 的 ``type``，默认 ``1``（单曲）
        :param limit: 返回数量，对应 ``Query`` 的 ``limit``，默认 ``30``
        :param offset: 偏移量，对应 ``Query`` 的 ``offset``，默认 ``0``

        未显式传入的参数不会写进 ``Query``，由 ``ncm-api-rs`` 自己套用默认值：
        ``type`` → ``1``、``limit`` → ``30``、``offset`` → ``0``；
        ``type == 2000`` 时走 ``/api/search/voice/get``。
        """

    def song_detail(
        self, ids: int | str | Iterable[int | str],
    ) -> Awaitable[ApiResponse]:
        """通过 id 获取歌曲信息，对应 ``ApiClient::song_detail``（``/song/detail``）.

        :param ids: 歌曲 id，支持 ``186016`` / ``"186016"`` /
            ``[186016, 186017]`` / ``"186016,186017"``

        字符串会被原样透传，切分与清洗由 ``ncm-api-rs`` 负责
        （按 ``,`` 切分、去空白、丢掉空串）。
        """


def create_client(cookie: str | None = None) -> Client:
    """创建一个新的 API 客户端，对应 ``ncm_api_rs::create_client``."""
