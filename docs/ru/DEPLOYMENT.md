# Деплой документации на GitLab Pages

## Настройка

### 1. Проверка файлов

Убедитесь, что все файлы созданы:

```bash
ls -la
# .gitlab-ci.yml
# mkdocs.yml
# docs/
```

### 2. Настройка GitLab CI/CD

#### Переменные окружения (опционально)

В GitLab UI перейдите в **Settings → CI/CD → Variables** и добавьте:

| Key | Value | Description |
|-----|-------|-------------|
| `TELEGRAM_BOT_TOKEN` | `your_bot_token` | Токен бота для уведомлений |
| `TELEGRAM_CHAT_ID` | `your_chat_id` | ID чата для уведомлений |

### 3. Пуш в репозиторий

```bash
git add .
git commit -m "Add documentation with GitLab Pages support"
git push origin main
```

## Автоматический деплой

После пуша в ветку `main`:

1. GitLab CI/CD автоматически запустит пайплайн
2. Job `pages` соберет документацию
3. Документация будет доступна по адресу:
   ```
   https://dark0ghost.gitlab.io/ai-trader-bot/
   ```

### Мониторинг деплоя

1. Перейдите в **CI/CD → Pipelines**
2. Кликните на последний пайплайн
3. Следите за статусом job `pages`

## Ручной деплой

### Локальная сборка

```bash
# Установка зависимостей
pip install -r docs/requirements.txt

# Сборка
mkdocs build --clean

# Проверка
ls site/
```

### Деплой конкретной версии

```bash
# Установка mike
pip install mike

# Настройка git
git config user.name "Your Name"
git config user.email "your.email@example.com"

# Деплой версии 0.2.0
mike deploy 0.2.0
mike deploy --push 0.2.0

# Установка версии по умолчанию
mike set-default 0.2.0
mike deploy --push --set-default 0.2.0
```

### Управление версиями

```bash
# Список версий
mike list

# Просмотр версий
mike serve

# Удаление версии
mike delete 0.1.0
```

## Проверка перед деплоем

### 1. Локальный предпросмотр

```bash
# Запуск локального сервера
mkdocs serve

# Открыть http://127.0.0.1:8000/
```

### 2. Строгая сборка

```bash
# Сборка с проверкой ошибок
mkdocs build --strict

# Проверка ссылок
mkdocs build --clean 2>&1 | grep -i "broken\|error"
```

### 3. Валидация конфигурации

```bash
# Проверка mkdocs.yml
mkdocs --config-file mkdocs.yml build --dry-run
```

## Troubleshooting

### Ошибка: "Config value 'theme': Unrecognised theme"

**Решение:**
```bash
pip install --upgrade mkdocs-material
```

### Ошибка: "Plugin error: minify"

**Решение:**
```bash
pip install mkdocs-minify-plugin
```

### Ошибка: "Page not found"

**Причина:** Неправильный путь в навигации

**Решение:**
Проверьте `mkdocs.yml`:
```yaml
nav:
  - Главная: index.md  # Путь относительно docs/
```

### Деплой не работает

**Проверка:**

1. Файл `.gitlab-ci.yml` существует
2. Job `pages` настроен правильно
3. Артефакты в папке `public/`

**Логи:**
```bash
# В GitLab UI: CI/CD → Pipelines → Jobs → pages
```

### Версии не переключаются

**Решение:**
```bash
# Сброс версий
git checkout gh-pages
git reset --hard HEAD~10
git push origin gh-pages --force

# Деплой заново
mike deploy --push 0.2.0
```

## Настройка домена

### Custom Domain

1. В GitLab UI: **Settings → Pages**
2. Введите домен: `docs.example.com`
3. Добавьте CNAME запись в DNS:
   ```
   docs.example.com. CNAME dark0ghost.gitlab.io.
   ```

### HTTPS

GitLab автоматически предоставляет HTTPS для GitLab Pages.

## Уведомления

### Telegram

Добавьте в `.gitlab-ci.yml`:

```yaml
notify:
  script:
    - curl -X POST "https://api.telegram.org/bot${TELEGRAM_BOT_TOKEN}/sendMessage" \
      -d "chat_id=${TELEGRAM_CHAT_ID}" \
      -d "text=Deployed: ${CI_PROJECT_NAME}"
```

### Email

GitLab автоматически отправляет email при неудачном деплое.

## Производительность

### Оптимизация сборки

```yaml
# .gitlab-ci.yml
pages:
  cache:
    paths:
      - .pip/
  before_script:
    - pip install --cache-dir .pip -r docs/requirements.txt
```

### Минификация

Включена в `mkdocs.yml`:

```yaml
plugins:
  - minify:
      minify_html: true
```

## Безопасность

### Секреты

Никогда не храните секреты в репозитории!

Используйте GitLab CI/CD Variables:
- **Settings → CI/CD → Variables**
- Тип: `Variable`
- Protected: ✅ (только для защищенных веток)
- Masked: ✅ (скрыть в логах)

### Доступ

Ограничьте доступ к репозиторию:
- **Settings → General → Visibility**
- Project visibility: `Private` или `Internal`

## Мониторинг

### Статистика посещений

Добавьте Google Analytics в `mkdocs.yml`:

```yaml
extra:
  analytics:
    provider: google
    property: G-XXXXXXXXXX
```

### Логи доступа

GitLab не предоставляет логи доступа для Pages.

Используйте внешние сервисы:
- Cloudflare Analytics
- Plausible Analytics
- Fathom Analytics

## Следующие шаги

1. ✅ Проверка локальной сборки
2. ✅ Пуш в репозиторий
3. ✅ Мониторинг пайплайна
4. ✅ Проверка опубликованной документации
5. ✅ Настройка уведомлений (опционально)

## Поддержка

- [GitLab Pages Documentation](https://docs.gitlab.com/ee/user/project/pages/)
- [MkDocs Material](https://squidfunk.github.io/mkdocs-material/)
- [mike Versioning](https://github.com/jimporter/mike)
