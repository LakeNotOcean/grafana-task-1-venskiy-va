#!/usr/bin/env python3
"""
Генератор дашборда Grafana для запущенного приложения на основе GET /handlers.
"""

import sys

from client import HttpClient
from config import GeneratorError, Settings, parse_args
from dashboard import build_dashboard
from grafana import publish_file, verify
from handlers import collect_handlers


def main() -> int:
    settings = Settings.resolve(parse_args())
    print(
        f"Grafana: {settings.grafana_url}",
        flush=True,
    )
    admin_auth = (settings.grafana_user, settings.grafana_password)

    # ждем
    HttpClient(settings.grafana_url, timeout=settings.timeout).get_json(
        "/api/health", attempts=settings.retries
    )

    # получаем список маршрутов метрик
    apps = [HttpClient(url, timeout=settings.timeout) for url in settings.app_urls]
    handlers = collect_handlers(apps, attempts=settings.retries)
    if not handlers:
        raise GeneratorError("приложение не отдало ни одного маршрута")

    # генерациия панелей
    dashboard = build_dashboard(handlers)

    grafana = HttpClient(settings.grafana_url, timeout=settings.timeout, auth=admin_auth)

    publish_file(grafana, dashboard, settings)

    # на всякий случай
    verify(grafana, settings)
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except GeneratorError as error:
        print(f"Ошибка: {error}", file=sys.stderr, flush=True)
        sys.exit(1)
