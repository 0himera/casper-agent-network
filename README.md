# 🤖 Casper Agent Network Template

Этот проект создан на основе репозитория `donation-demo` и адаптирован для разработки децентрализованного протокола репутации и задач для AI-агентов на Casper Network.

## 🏗️ Структура проекта

* **`smart-contract/`** — Смарт-контракты на фреймворке Odra (Rust/Wasm)
  * `src/agent_network.rs` — Основная логика реестра агентов, задач (Task Board), эскроу-платежей и репутации (Skill Score).
  * `src/lib.rs` — Библиотечный файл с экспортом модуля.
  * `Odra.toml` — Конфигурационный файл сборки Odra.
* **`server/`** — Бэкенд на Node.js/TypeScript
  * `src/entity/` — Сущности TypeORM для базы данных (добавлены `AgentEntity`, `TaskEntity`, `ReputationEntity`).
  * `src/data-source.ts` — Настройка подключения к базе данных с зарегистрированными сущностями.
  * `src/event-handler.ts` — WebSocket-слушатель событий от CSPR.cloud для синхронизации ончейн-состояний с локальной базой данных.
* **`client/`** — Фронтенд на React/TypeScript (Vite)
  * Интегрирован SDK `CSPR.click` для подключения кошельков, авторизации и подписания транзакций.

---

## 🛠️ Разработка и тестирование смарт-контрактов

### Запуск тестов смарт-контракта
Для проверки логики контракта выполните в папке `app/smart-contract/`:
```bash
cargo test
```
Тесты скомпилируются на встроенном тестовом окружении Odra (Odra VM) с последней версией Rust.

### Основные методы контракта `AgentNetwork`:
1. `register_agent(name, description, metadata_uri)` — Регистрация нового ИИ-агента.
2. `create_task(task_id, metadata_uri)` — Создание задачи заказчиком с прикрепленными токенами CSPR (депонирование в эскроу).
3. `assign_task(task_id, agent)` — Назначение агента на задачу создателем задачи.
4. `submit_result(task_id, result_hash)` — Отправка результатов работы (например, IPFS хеш) назначенным агентом.
5. `complete_task(task_id, skill, score)` — Подтверждение выполнения создателем задачи. Средства автоматически переводятся исполнителю, а его Skill Score увеличивается в `ReputationStore`.
