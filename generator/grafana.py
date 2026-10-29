"""Экспорт дашборда в Grafana"""

import json
import os

from client import HttpClient
from config import GeneratorError, Settings
from dashboard import DASHBOARD_UID


def publish_file(client: HttpClient, dashboard: dict, settings: Settings) -> None:
    """запись в каталог"""
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
    """перезапуск дашборда"""
    try:
        client.post_json("/api/admin/provisioning/dashboards/reload")
    except OSError as error:
        raise GeneratorError(f"Grafana недоступна при перезагрузке provisioning: {error}") from error


def verify(client: HttpClient, settings: Settings) -> None:
    """проверка загруженного дашборда"""
    payload = client.get_json(
        f"/api/dashboards/uid/{DASHBOARD_UID}", attempts=settings.retries
    )

    dashboard = payload.get("dashboard", {})
    meta = payload.get("meta", {})
    print(
        "проверка: "
        f"uid={dashboard.get('uid')!r} title={dashboard.get('title')!r} "
        f"panels={len(dashboard.get('panels', []))} "
        f"editable={dashboard.get('editable')} provisioned={meta.get('provisioned')} "
        f"canEdit={meta.get('canEdit')} canSave={meta.get('canSave')}",
        flush=True,
    )
