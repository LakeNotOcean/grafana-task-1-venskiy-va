"""Конфигурация генератора"""

import argparse
import os
from dataclasses import dataclass

DEFAULT_APP_URLS = "http://app-dc1:8080"
DEFAULT_GRAFANA_URL = "http://grafana:3000"
DEFAULT_GRAFANA_USER = "admin"
DEFAULT_DASHBOARD_FILE = "/var/lib/grafana/dashboards/app-handlers.json"
DEFAULT_TIMEOUT_SECONDS = 5.0
DEFAULT_RETRIES = 60


class GeneratorError(RuntimeError):
    """Ошибка генератора"""


def _pick(cli_value, env_name: str, default, cast=str):
    """Значение из CLI, иначе из env, иначе дефолт."""
    if cli_value is not None:
        return cast(cli_value)
    return cast(os.environ.get(env_name, default))


@dataclass(frozen=True)
class Settings:
    app_urls: tuple[str, ...]
    grafana_url: str
    grafana_user: str
    grafana_password: str
    dashboard_file: str
    timeout: float
    retries: int

    @classmethod
    def resolve(cls, args: argparse.Namespace) -> "Settings":
        raw_urls = args.app_url or os.environ.get("APP_URLS", DEFAULT_APP_URLS).split(",")
        app_urls = tuple(url.strip().rstrip("/") for url in raw_urls if url.strip())
        if not app_urls:
            raise GeneratorError("список адресов приложения пуст")

        # Пароль — только из env: аргумент попал бы в историю оболочки и в ps.
        password = os.environ.get("GRAFANA_ADMIN_PASSWORD")
        if not password:
            raise GeneratorError(
                "GRAFANA_ADMIN_PASSWORD не задан"
            )

        return cls(
            app_urls=app_urls,
            grafana_url=_pick(args.grafana_url, "GRAFANA_URL", DEFAULT_GRAFANA_URL).rstrip("/"),
            grafana_user=os.environ.get("GRAFANA_ADMIN_USER", DEFAULT_GRAFANA_USER),
            grafana_password=password,
            dashboard_file=_pick(args.dashboard_file, "DASHBOARD_FILE", DEFAULT_DASHBOARD_FILE),
            timeout=_pick(args.timeout, "HTTP_TIMEOUT_SECONDS", DEFAULT_TIMEOUT_SECONDS, float),
            retries=_pick(args.retries, "HTTP_RETRIES", DEFAULT_RETRIES, int),
        )


def parse_args() -> argparse.Namespace:
    """необязательные окружения"""
    parser = argparse.ArgumentParser(
        description="Генерирует дашборд Grafana по списку ручек запущенного приложения."
    )
    parser.add_argument(
        "--app-url",
        action="append",
        metavar="URL",
        help="адрес экземпляра приложения; можно передать несколько (env: APP_URLS через запятую)",
    )
    parser.add_argument(
        "--grafana-url", metavar="URL", help="адрес Grafana (env: GRAFANA_URL)"
    )
    parser.add_argument(
        "--dashboard-file",
        metavar="PATH",
        help="путь к файлу дашборда для режима file (env: DASHBOARD_FILE)",
    )
    parser.add_argument(
        "--timeout",
        type=float,
        metavar="SEC",
        help="таймаут одного HTTP-запроса, секунд (env: HTTP_TIMEOUT_SECONDS)",
    )
    parser.add_argument(
        "--retries",
        type=int,
        metavar="N",
        help="число попыток ожидания готовности, пауза 2 с (env: HTTP_RETRIES)",
    )
    return parser.parse_args()
