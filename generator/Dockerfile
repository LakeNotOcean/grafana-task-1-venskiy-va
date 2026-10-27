FROM python:3.14-alpine

WORKDIR /generator
COPY *.py ./

# Однократный скрипт дял генерации
CMD ["python", "-u", "generate_dashboard.py"]
