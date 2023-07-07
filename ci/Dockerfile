# Тест приложения
# Окружение - локальное

# Сборка пакета
FROM rust:latest as builder
WORKDIR /app

# Клонируем репозиторий
RUN git clone https://github.com/harlanc/xiu.git

# Собираем и устанавливаем пакет
WORKDIR /app/xiu
RUN cargo build --release

# Финальный этап
FROM alpine:latest
WORKDIR /app
# Копируем только бинарный файл из предыдущего этапа
COPY --from=builder /app/xiu/target/release/xiu /app
COPY --from=builder /app/xiu/application/xiu/src/config/config_rtmp.toml /app

# Запускаем пакет
CMD ["xiu", "-c", "/app/xiu/config/config_rtmp.toml"]
