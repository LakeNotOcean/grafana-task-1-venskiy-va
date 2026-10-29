"""
Модель дашборда Grafana
"""

from dataclasses import dataclass

from promql import REQUESTS_METRIC, errors_expr, latency_expr, rps_expr

DATASOURCE = {"type": "prometheus", "uid": "prom-ds"}
DASHBOARD_UID = "app-handlers"
DASHBOARD_TITLE = "Application handlers"
QUANTILES = (("0.5", "50"), ("0.95", "95"), ("0.99", "99"))

ROW_HEIGHT = 1
PANEL_HEIGHT = 8
GROUP_HEIGHT = ROW_HEIGHT + PANEL_HEIGHT
PANEL_WIDTH = 8


@dataclass(frozen=True)
class PanelSpec:
    """Спецификация одной панели: заголовок, источиник, единицы, описание"""

    title: str
    targets: tuple[tuple[str, str], ...]
    unit: str
    description: str


def panel_specs(method: str | None, handler: str | None) -> list[PanelSpec]:
    """три панели: задержка, запросы и ошибки"""
    suffix = "" if handler is None else f": {method} {handler}"
    return [
        PanelSpec(
            f"Latency p50/p95/p99{suffix}",
            tuple(
                (latency_expr(method, handler, quantile), f"p{label}")
                for quantile, label in QUANTILES
            ),
            "s",
            "Перцентили по бакетам",
        ),
        PanelSpec(
            f"RPS{suffix}",
            ((rps_expr(method, handler), "rps"),),
            "reqps",
            "Запросов в секунду на основе счетчика запросов",
        ),
        PanelSpec(
            f"Errors by code{suffix}",
            ((errors_expr(method, handler), "{{code}}"),),
            "reqps",
            "Ошибки 4xx и 5xx",
        ),
    ]


def _row(panel_id: int, title: str, y: int) -> dict:
    return {
        "id": panel_id,
        "type": "row",
        "title": title,
        "collapsed": False,
        "gridPos": {"h": ROW_HEIGHT, "w": 24, "x": 0, "y": y},
        "panels": [],
    }


def _timeseries(panel_id: int, spec: PanelSpec, grid_pos: dict[str, int]) -> dict:
    return {
        "id": panel_id,
        "type": "timeseries",
        "title": spec.title,
        "description": spec.description,
        "datasource": DATASOURCE,
        "gridPos": grid_pos,
        "targets": [
            {
                "refId": chr(ord("A") + index),
                "expr": expr,
                "legendFormat": legend,
                "datasource": DATASOURCE,
            }
            for index, (expr, legend) in enumerate(spec.targets)
        ],
        "fieldConfig": {
            "defaults": {
                "unit": spec.unit,
                "color": {"mode": "palette-classic"},
                "custom": {
                    "drawStyle": "line",
                    "lineWidth": 1,
                    "fillOpacity": 10,
                    "spanNulls": True,
                    "showPoints": "never",
                },
            },
            "overrides": [],
        },
        "options": {
            "legend": {
                "displayMode": "table",
                "placement": "bottom",
                "showLegend": True,
                "calcs": ["mean", "max", "lastNotNull"],
            },
            "tooltip": {"mode": "multi", "sort": "desc"},
        },
    }


def _query_var(name: str, label: str, query: str) -> dict:
    """селектор"""
    return {
        "name": name,
        "label": label,
        "type": "query",
        "datasource": DATASOURCE,
        "query": {"query": query, "refId": name},
        "definition": query,
        "refresh": 1,
        "sort": 1,
        "regex": "",
        "multi": True,
        "includeAll": True,
        "allValue": ".*",
        "current": {"text": ["All"], "value": ["$__all"]},
    }


def _variables() -> list[dict]:
    """селекторы dc и pod"""
    return [
        _query_var("dc", "Datacenter", f"label_values({REQUESTS_METRIC}, dc)"),
        _query_var("pod", "Pod", f'label_values({REQUESTS_METRIC}{{dc=~"$dc"}}, pod)'),
    ]


def build_dashboard(handlers: list[dict[str, str]]) -> dict:
    """группа панелей на каждый маршрут"""
    groups = [("All Routes", None, None)] + [
        (f"{h['method']} {h['path']}. {h['description']}", h["method"], h["path"])
        for h in handlers
    ]

    panels: list[dict] = []
    for group, (title, method, handler) in enumerate(groups):
        y = group * GROUP_HEIGHT
        panels.append(_row(len(panels) + 1, title, y))
        for column, spec in enumerate(panel_specs(method, handler)):
            grid_pos = {"h": PANEL_HEIGHT, "w": PANEL_WIDTH, "x": column * PANEL_WIDTH, "y": y + ROW_HEIGHT}
            panels.append(_timeseries(len(panels) + 1, spec, grid_pos))

    return {
        "uid": DASHBOARD_UID,
        "title": DASHBOARD_TITLE,
        "tags": ["items-service", "generated"],
        "timezone": "browser",
        "editable": False,
        "graphTooltip": 1,
        "refresh": "10s",
        "schemaVersion": 39,
        "version": 0,
        "time": {"from": "now-15m", "to": "now"},
        "templating": {"list": _variables()},
        "annotations": {"list": []},
        "panels": panels,
    }
