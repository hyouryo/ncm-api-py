"""ncm-api-py 的类型标注（Type stubs）.

对 ``ncm-api-rs`` 的 PyO3 绑定，目前只包含三个接口：

- :meth:`Client.search` —— 歌曲搜索
- :meth:`Client.song_detail` —— 通过 id 获取歌曲信息
- :meth:`Client.song_url_v1` —— 通过 id 获取歌曲播放链接

三者都是 ``async`` 方法，成功时返回 :class:`ApiResponse`。
"""

from collections.abc import Awaitable

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
    def body(self) -> dict:
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
        self, ids: list[int],
    ) -> Awaitable[ApiResponse]:
        """通过 id 获取歌曲信息，对应 ``ApiClient::song_detail``（``/song/detail``）.

        :param ids: 歌曲 id
            必须是 ``int`` 或 ``str``，其余类型（含 ``bytes``/``bytearray``）
            会抛 ``TypeError``
        """

    def song_url_v1(
        self, ids: list[int], level: str | None = None,
    ) -> Awaitable[ApiResponse]:
        """通过 id 获取歌曲播放链接，对应 ``ApiClient::song_url_v1``.

        即 ``/song/url/v1``；链接在 ``body["data"][i]["url"]``。
        无版权或需要付费时该字段可能是 ``None``，但**状态码仍是 200**，
        调用方必须自己判空。

        :param ids: 歌曲 id
            必须是 ``str``，其余类型会抛 ``TypeError``
        :param level: 音质，对应 ``Query`` 的 ``level``，默认 ``standard``
            （另有 ``higher``/``exhigh``/``lossless``/``hires``/``sky`` 等）
        """


def create_client(cookie: str | None = None) -> Client:
    """创建一个新的 API 客户端，对应 ``ncm_api_rs::create_client``."""
