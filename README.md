# Filename Cleaner

Простая программа на Rust для Windows, которая очищает имена файлов в выбранной папке от эмодзи и специальных символов.

## Что делает

- Открывает диалог выбора папки
- Для каждого файла в папке (без рекурсии в подпапки):
  - Удаляет эмодзи и прочие спецсимволы
  - Сохраняет буквы (включая акценты: é, à, ç и т.д.), цифры, пробелы и безопасную пунктуацию
  - Нормализует «красивые» кавычки и тире
  - Убирает лишние пробелы
- Показывает список изменений и просит подтверждение
- При конфликтах имён (два файла стали одинаковыми) добавляет `(1)`, `(2)` и т.д.

## CI / Релизы

При пуше тега `v*` (например `v0.1.0`) GitHub Actions собирает бинарники и автоматически прикрепляет их к релизу:

- `filename_cleaner-windows-amd64.exe`
- `filename_cleaner-linux-amd64`
- `filename_cleaner-linux-arm64`

Создать релиз:

```bash
git tag v0.1.0
git push origin v0.1.0
```

## Сборка

### Windows (с диалогом выбора папки)

```bash
cargo build --release --features gui
```

Готовый файл: `target/release/filename_cleaner.exe`

### Linux (с диалогом выбора папки)

```bash
# нужны системные пакеты для GTK и Wayland
sudo apt-get install -y libgtk-3-dev pkg-config libwayland-dev
cargo build --release
./target/release/filename_cleaner
```

Запуск без аргументов открывает диалог. Путь можно также передать аргументом.

## Запуск

Просто запустите `filename_cleaner.exe` — появится диалог выбора папки.

## Пример

До:
```
38. Fly me to the moon - Frank Sinatra 🌙  (French Version   Chloé Stafler).mp4
```

После:
```
38. Fly me to the moon - Frank Sinatra (French Version Chloé Stafler).mp4
```

## Тесты

Логика очистки покрыта unit-тестами в `src/main.rs` (модуль `tests`).

На Linux/macOS:
```bash
cargo test
```

Ключевые случаи (эмодзи, акценты, нормализация пробелов, «красивые» кавычки, trailing dots) проверяются автоматически.
