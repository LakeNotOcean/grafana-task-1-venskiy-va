"""
Клиент для взаимодействия Grafana
"""

import base64
import json
import time
import urllib.error
import urllib.request

from config import GeneratorError

RETRY_DELAY_SECONDS = 2.0


class HttpClient:
    """Клиент с базовым url и авторизацией"""

    def __init__(self, base_url: str, timeout: float, auth: tuple[str, str] | None = None):
        self.base_url = base_url.rstrip("/")
        self.timeout = timeout
        self.auth = auth

    def _headers(self, json_body: bool) -> dict[str, str]:
        headers = {"Accept": "application/json"}
        if json_body:
            headers["Content-Type"] = "application/json"
        if self.auth:
            credentials = f"{self.auth[0]}:{self.auth[1]}".encode()
            headers["Authorization"] = f"Basic {base64.b64encode(credentials).decode()}"
        return headers

    def _exchange(self, path: str, payload: bytes | None, method: str) -> tuple[int, bytes]:
        request = urllib.request.Request(
            self.base_url + path,
            data=payload,
            headers=self._headers(json_body=payload is not None),
            method=method,
        )
        try:
            with urllib.request.urlopen(request, timeout=self.timeout) as response:
                return response.status, response.read()
        except urllib.error.HTTPError as error:
            return error.code, error.read()

    def get_json(self, path: str, attempts: int = 1) -> object:
        url = self.base_url + path
        last_error = "нет ответа"
        for attempt in range(1, attempts + 1):
            try:
                status, body = self._exchange(path, payload=None, method="GET")
                if status == 200:
                    return json.loads(body)
                last_error = f"HTTP {status}: {body[:200].decode(errors='replace')}"
            except (urllib.error.URLError, TimeoutError, OSError) as error:
                last_error = str(error)
            print(f"  [{attempt}/{attempts}] {url} — {last_error}", flush=True)
            time.sleep(RETRY_DELAY_SECONDS)

        raise GeneratorError(f"не дождался ответа от {url}: {last_error}")

    def post_json(self, path: str, obj: object | None = None) -> tuple[int, bytes]:
        payload = None if obj is None else json.dumps(obj, ensure_ascii=False).encode()
        return self._exchange(path, payload=payload, method="POST")
