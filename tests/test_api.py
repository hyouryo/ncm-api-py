"""Offline public API checks: no credentials or live service required."""

import unittest

import ncm_api_py


class Indexable:
    def __index__(self) -> int:
        return 186016


class ClientTests(unittest.TestCase):
    def setUp(self) -> None:
        self.client = ncm_api_py.create_client()

    def test_public_endpoints(self) -> None:
        methods = {name for name in vars(ncm_api_py.Client) if not name.startswith("_")}
        self.assertEqual(methods, {"search", "song_detail", "song_url_v1"})
        self.assertEqual(
            set(ncm_api_py.__all__), {"ApiResponse", "Client", "create_client"}
        )
        self.assertIsInstance(ncm_api_py.Client(cookie=None), ncm_api_py.Client)
        self.assertIsInstance(ncm_api_py.create_client(cookie=""), ncm_api_py.Client)
        # Annotations can parameterize the response at runtime too.
        self.assertIs(ncm_api_py.ApiResponse[dict[str, object]], ncm_api_py.ApiResponse)

    def test_cookie_type(self) -> None:
        for cookie in (1, False, b"cookie", []):
            with self.subTest(cookie=cookie), self.assertRaises(TypeError):
                ncm_api_py.create_client(cookie)
            with self.subTest(cookie=cookie), self.assertRaises(TypeError):
                ncm_api_py.Client(cookie)

    def test_search_argument_types(self) -> None:
        for value in (False, True, 1.0, "1", Indexable()):
            for parameter in ("type", "limit", "offset"):
                with self.subTest(parameter=parameter, value=value):
                    with self.assertRaises(TypeError):
                        self.client.search("music", **{parameter: value})
        for keywords in (None, 1, b"music"):
            with self.subTest(keywords=keywords), self.assertRaises(TypeError):
                self.client.search(keywords)

    def test_search_argument_values(self) -> None:
        for kwargs in (
            {"type": 0},
            {"type": 2},
            {"type": 9999},
            {"limit": 0},
            {"limit": -1},
            {"offset": -1},
        ):
            with self.subTest(kwargs=kwargs), self.assertRaises(ValueError):
                self.client.search("music", **kwargs)
        for parameter in ("type", "limit", "offset"):
            with self.subTest(parameter=parameter), self.assertRaises(OverflowError):
                self.client.search("music", **{parameter: 2**63})

    def test_song_id_types(self) -> None:
        invalid_ids = (
            186016,
            "186016,186017",
            (186016,),
            {186016},
            None,
            [True],
            [False],
            ["186016"],
            [186016.0],
            [Indexable()],
            [186016, None],
        )
        for method in (self.client.song_detail, self.client.song_url_v1):
            for ids in invalid_ids:
                with self.subTest(method=method, ids=ids), self.assertRaises(TypeError):
                    method(ids)

    def test_song_id_values(self) -> None:
        for method in (self.client.song_detail, self.client.song_url_v1):
            for ids in ([], [0], [-1], [186016, 0]):
                with (
                    self.subTest(method=method, ids=ids),
                    self.assertRaises(ValueError),
                ):
                    method(ids)
            with self.assertRaises(OverflowError):
                method([2**63])

    def test_quality_type_and_values(self) -> None:
        for level in (1, False, b"standard", []):
            with self.subTest(level=level), self.assertRaises(TypeError):
                self.client.song_url_v1([186016], level=level)
        for level in ("", "invalid", "STANDARD"):
            with self.subTest(level=level), self.assertRaises(ValueError):
                self.client.song_url_v1([186016], level=level)

    def test_valid_arguments_require_event_loop(self) -> None:
        # future_into_py looks up the running loop before spawning its task.
        # Thus these exercise argument acceptance without issuing HTTP requests.
        for search_type in (
            None,
            1,
            10,
            100,
            1000,
            1002,
            1004,
            1006,
            1009,
            1014,
            1018,
            2000,
        ):
            with self.subTest(type=search_type):
                with self.assertRaisesRegex(RuntimeError, "no running event loop"):
                    self.client.search(
                        "music", type=search_type, limit=None, offset=None
                    )
        with self.assertRaisesRegex(RuntimeError, "no running event loop"):
            self.client.search("music", limit=1, offset=0)
        with self.assertRaisesRegex(RuntimeError, "no running event loop"):
            self.client.song_detail([186016, 186017])
        for level in (
            None,
            "standard",
            "higher",
            "exhigh",
            "lossless",
            "hires",
            "jyeffect",
            "sky",
            "dolby",
            "jymaster",
        ):
            with self.subTest(level=level):
                with self.assertRaisesRegex(RuntimeError, "no running event loop"):
                    self.client.song_url_v1([186016, 186017], level=level)


if __name__ == "__main__":
    unittest.main()
