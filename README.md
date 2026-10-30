# Описание

- CRUD-микросервис на Rust, который отдаёт Prometheus-метрики: latency, traffic, errors. 
- Генератор дашборда Grafana на основе данных метода `GET /handlers`. 
- Также реализована фоновая нагрузка с k6.
 

## Запуск

```bash
cp .env.example .env
docker compose up
```

После генерации дашборда данные видны в Grafana: <http://localhost:3000/d/app-handlers>
(вход через `GRAFANA_ADMIN_USER` / `GRAFANA_ADMIN_PASSWORD` из `.env`). Дашборд
"Application handlers" находится в корне списка дашбордов. На дашборде есть селекторы `dc` и `pod`, а панели агрегируют
метрики по выбранным значениям.

## Состав

| Сервис compose | Образ | Назначение |
| --- | --- | --- |
| `app-dc1` | собран из `./Dockerfile` (`items-service:dc1`) | экземпляр приложения, `POD_ID=pod-1`, `DATACENTER=dc1`, порт 8081 |
| `app-dc2` | тот же Dockerfile (`items-service:dc2`) | экземпляр приложения, `POD_ID=pod-2`, `DATACENTER=dc2`, порт 8082 |
| `prometheus` | `prom/prometheus:v3.15.1` | берет данные из обоих экземпляров каждые 5 секунд |
| `grafana` | `grafana/grafana-oss:12.4.3` | datasource и дашборды через provisioning |
| `dashboard-generator` | на базе `python:3.14-alpine` | читает `/handlers` и добавляет дашборд через file-provisioning |
| `load-generator` | `grafana/k6:2.3.0` | постоянная фоновая нагрузка на оба экземпляра |

## Порты

| Порт | Ресурс |
| --- | --- |
| `3000` | Grafana UI |
| `9090` | Prometheus UI |
| `8081` | `app-dc1` (внутри сети - `8080`) |
| `8082` | `app-dc2` (внутри сети - `8080`) |

- Основные маршруты: `POST /items` (201), `GET /items/{id}` (200 / 404), `PUT /items/{id}` (200 / 404).
- Служебные: `GET /health`, `GET /metrics`, `GET /handlers`.

## Метрики и метки

- `http_request_duration_seconds` - Гистограмма. Метки `method`, `handler` на основе данных `GET /handlers` и постоянные `pod`, `dc` для Latency p50/p95/p99.
- `http_requests_total` - Counter. Метки `method`, `handler` на основе `/handlers`, `code` и постоянные `pod`, `dc` для RPS и ошибок по кодам.
- Служебные `/health`, `/metrics`.
- `pod`/`dc` из `POD_ID`/`DATACENTER`.
- Бакеты для гистограммы: 15 границ от 100 мкс до 5 секунд (`LATENCY_BUCKETS` в `src/metrics.rs`).

Использовался метод Histogram, а не Summary для latency, так как Summary вычисляет неагрегируемые квантили внутри отдельного пода, а при Histogram вычисляются аддитивные бакеты. Перцентиль, в свою очередь, приблизительно восстанавливается по объединенной выборке. Например:
`histogram_quantile(0.95, sum by (le) (rate(http_request_duration_seconds_bucket[$__rate_interval])))`.
Здесь `sum by (le)` суммирует бакеты выбранным подам и дата-центрам.

## Нагрузка

Для генерации нагрузки используется k6. На оба экземпляра приходится суммарно приблизительнно 36 RPS. Операции распределяются следующим образом: 30 % POST, 35 % GET, 30 % PUT, 5 % GET несуществующего id (ошибка 404). Каждый `FAIL_EVERY_N`-й запрос сервиса с ошибкой 500.

## Генератор дашборда

Для генерации дашборда используется скрипт на Python. Точка входа - `generator/generate_dashboard.py`.

## Переменные окружения

Переменные окружения хранятся в `.env`, пример описан в `.env.example`:
- GRAFANA_ADMIN_USER - пользователь Grafana
- GRAFANA_ADMIN_PASSWORD - пароль Grafana
- FAIL_EVERY_N - частота искусственного сбоя

## Использование ИИ

Огромное спасибо дорогому Qwen за конфигурации, исправление ошибок и рутинный вспомогательный код.
