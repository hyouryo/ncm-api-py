"""Three asynchronous endpoints; body schemas are static typing helpers only."""

from collections.abc import Awaitable
from typing import Generic, Literal, TypeAlias, TypedDict, TypeVar, overload

__all__ = ["ApiResponse", "Client", "create_client"]

JSONValue: TypeAlias = (
    None | bool | int | float | str | list[JSONValue] | dict[str, JSONValue]
)
JSONObject: TypeAlias = dict[str, JSONValue]
SearchType: TypeAlias = Literal[
    1, 10, 100, 1000, 1002, 1004, 1006, 1009, 1014, 1018, 2000
]
OtherSearchType: TypeAlias = Literal[
    10, 100, 1000, 1002, 1004, 1006, 1009, 1014, 1018, 2000
]
QualityLevel: TypeAlias = Literal[
    "standard",
    "higher",
    "exhigh",
    "lossless",
    "hires",
    "jyeffect",
    "sky",
    "dolby",
    "jymaster",
]

class Artist(TypedDict):
    id: int
    name: str

class Album(TypedDict):
    id: int
    name: str

class _SearchSongOptional(TypedDict, total=False):
    artists: list[Artist]
    album: Album
    duration: int
    alias: list[str]

class SearchSong(_SearchSongOptional):
    id: int
    name: str

class SongSearchResult(TypedDict, total=False):
    # Empty results can omit songs altogether.
    songs: list[SearchSong]
    songCount: int
    hasMore: bool

class SongSearchBody(TypedDict):
    code: int
    result: SongSearchResult

class _SongOptional(TypedDict, total=False):
    ar: list[Artist]
    al: Album
    dt: int
    alia: list[str]
    fee: int
    mv: int

class Song(_SongOptional):
    id: int
    name: str

class _SongDetailOptional(TypedDict, total=False):
    privileges: list[JSONObject]

class SongDetailBody(_SongDetailOptional):
    code: int
    songs: list[Song]

class _SongUrlOptional(TypedDict, total=False):
    br: int
    size: int
    md5: str | None
    type: str | None
    level: QualityLevel
    fee: int
    payed: int
    time: int
    expi: int
    freeTrialInfo: JSONObject | None

class SongUrl(_SongUrlOptional):
    id: int
    url: str | None
    code: int

class SongUrlBody(TypedDict):
    code: int
    data: list[SongUrl]

_BodyT_co = TypeVar("_BodyT_co", covariant=True)

class ApiResponse(Generic[_BodyT_co]):
    @property
    def status(self) -> int: ...
    @property
    def body(self) -> _BodyT_co: ...
    @property
    def cookie(self) -> list[str]: ...

class Client:
    def __init__(self, cookie: str | None = None) -> None: ...
    @overload
    def search(
        self,
        keywords: str,
        type: Literal[1] | None = None,
        limit: int | None = None,
        offset: int | None = None,
    ) -> Awaitable[ApiResponse[SongSearchBody]]: ...
    @overload
    def search(
        self,
        keywords: str,
        type: OtherSearchType,
        limit: int | None = None,
        offset: int | None = None,
    ) -> Awaitable[ApiResponse[JSONObject]]: ...
    @overload
    def search(
        self,
        keywords: str,
        type: SearchType | None,
        limit: int | None = None,
        offset: int | None = None,
    ) -> Awaitable[ApiResponse[SongSearchBody] | ApiResponse[JSONObject]]: ...
    def song_detail(self, ids: list[int]) -> Awaitable[ApiResponse[SongDetailBody]]: ...
    def song_url_v1(
        self, ids: list[int], level: QualityLevel | None = None
    ) -> Awaitable[ApiResponse[SongUrlBody]]: ...

def create_client(cookie: str | None = None) -> Client: ...
