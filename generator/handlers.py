"""Сбор GET /handlers"""

from client import HttpClient
from config import GeneratorError


def collect_handlers(clients: list[HttpClient], attempts: int) -> list[dict[str, str]]:
    """методы в виде пары (method, path)."""
    merged: dict[tuple[str, str], dict[str, str]] = {}
    for client in clients:
        payload = client.get_json("/handlers", attempts=attempts)
        entries = payload.get("handlers") if isinstance(payload, dict) else payload
        if not isinstance(entries, list):
            raise GeneratorError(
                f"{client.base_url}/handlers некорретный формат: {payload!r}"
            )

        for entry in entries:
            key = (str(entry["method"]), str(entry["path"]))
            merged[key] = {
                "method": key[0],
                "path": key[1],
                "description": str(entry.get("description", "")),
            }
        print(f"  {client.base_url}: {len(entries)} ручек", flush=True)

    return [merged[key] for key in sorted(merged)]
