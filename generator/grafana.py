"""Экспорт дашборда в Grafana"""

import json
import os

from client import HttpClient
from config import GeneratorError, Settings
from dashboard import DASHBOARD_UID


def publish_api(client: HttpClient, dashboard: dict) -> None:
    """REST API"""
    payload = {
        "dashboard": dashboard,
        "overwrite": True,
        "message": "сгенерировано generate_dashboard.py",
    }
    try:
        status, body = client.post_json("/api/dashboards/db", payload)
    except OSError as error:
        raise GeneratorError(f"Grafana недоступна при создании дашборда: {error}") from error

    if status not in (200, 201):
        raise GeneratorError(
            f"Grafana отклонила дашборд: HTTP {status}: {body[:500].decode(errors='replace')}"
        )

    url = json.loads(body).get("url", "") if body else ""
    print(f"дашборд создан через API: {url}", flush=True)


def publish_file(client: HttpClient, dashboard: dict, settings: Settings) -> None:
    """В каталог"""
    directory = os.path.dirname(settings.dashboard_file)
    if directory:
        os.makedirs(directory, exist_ok=True)

    temporary = f"{settings.dashboard_file}.tmp"
    with open(temporary, "w", encoding="utf-8") as handle:
        json.dump(dashboard, handle, ensure_ascii=False, indent=2)
    os.replace(temporary, settings.dashboard_file)

    print(f"дашборд записан в {settings.dashboard_file}", flush=True)
    reload_provisioning(client)


def reload_provisioning(client: HttpClient) -> None:
    """Перезапуск каталога"""
    try:
        status, body = client.post_json("/api/admin/provisioning/dashboards/reload")
    except OSError as error:
        raise GeneratorError(f"Grafana недоступна при перезагрузке provisioning: {error}") from error

    if status == 200:
        print("Grafana перечитала provisioning-каталог", flush=True)
        return


def verify(client: HttpClient, settings: Settings) -> None:
    """Проверка загруженного дашборда"""
    payload = client.get_json(
        f"/api/dashboards/uid/{DASHBOARD_UID}", attempts=settings.retries
    )

    dashboard = payload.get("dashboard", {})
    meta = payload.get("meta", {})
    print(
        "  проверка: "
        f"uid={dashboard.get('uid')!r} title={dashboard.get('title')!r} "
        f"panels={len(dashboard.get('panels', []))} "
        f"editable={dashboard.get('editable')} provisioned={meta.get('provisioned')} "
        f"canEdit={meta.get('canEdit')} canSave={meta.get('canSave')}",
        flush=True,
    )
