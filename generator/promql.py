"""Шаблоны PromQL-запросов"""

REQUESTS_METRIC = "total_requests"
LATENCY_BUCKETS = "request_duration_seconds_bucket"
RATE_WINDOW = "$__rate_interval"


def selector(method: str | None, handler: str | None) -> str:
    """Фильтр серий для pod и dc"""
    parts = ['dc=~"$dc"', 'pod=~"$pod"']
    if handler is not None:
        parts.insert(0, f'handler="{handler}"')
    if method is not None:
        parts.insert(0, f'method="{method}"')
    return ", ".join(parts)


def latency_expr(method: str | None, handler: str | None, quantile: str) -> str:
    """Перцентиль латентности"""
    buckets = f"rate({LATENCY_BUCKETS}{{{selector(method, handler)}}}[{RATE_WINDOW}])"
    return f"histogram_quantile({quantile}, sum by (le) ({buckets}))"


def rps_expr(method: str | None, handler: str | None) -> str:
    """Счетчик запросов"""
    return f"sum(rate({REQUESTS_METRIC}{{{selector(method, handler)}}}[{RATE_WINDOW}]))"


def errors_expr(method: str | None, handler: str | None) -> str:
    """Счетчик по кодам, в данном случае по ошибкам"""
    sel = f'{selector(method, handler)}, code=~"4..|5.."'
    return f"sum by (code) (rate({REQUESTS_METRIC}{{{sel}}}[{RATE_WINDOW}]))"
