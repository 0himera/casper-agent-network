# Методика оценки

Принципы скоринга для validator-engine. Подробный план доработок: [roadmap.md](./roadmap.md).

---

## Базовый принцип

Базовая единица — **измеримые флаги и баллы по рубрике**, а не «лучший ответ». Deterministic checks для формализуемых условий; LLM-as-judge — только для интерпретации, с фиксированными labels → score в коде.

```
input → gates → deterministic checks → rubric scoring → aggregation → pass/fail
```

Pairwise — только для калибровки весов, не как финальный выход.

---

## Целевой pipeline (реализован)

```
gate → код-оракул (hard-критерии) → LLM enum-label (soft, temp=0) → код-агрегация + threshold
```

1. **Hard gate + checklist rubric** — формат, sanity, no-leakage
2. **Weighted thresholded rubric** — веса + порог прохождения (≥ 70)
3. **Code verifier + rubric fallback** — числа/правила в коде; LLM → enum labels (`strong` / `partial` / `missing`) для soft-критериев; score считает код

Soft label mapping: `strong` → 100% weight, `partial` → 50%, `missing` → 0%.

**Текущее состояние:** pipeline реализован; 11 real tools — authoritative scorer для hard-критериев; few-shot exemplars, per-skill judge routing и optional self-consistency добавлены поверх baseline.

---

## Приоритеты внедрения

| P | Механизм | Статус |
|---|----------|--------|
| **P0** | Real tools as authoritative scorer | ✅ |
| **P0** | `temperature=0` on judge LLM | ✅ |
| **P0** | Input gates → skip LLM on hard fail | ✅ |
| **P0** | LLM → enum labels; score in code | ✅ |
| **P0** | Few-shot exemplars for soft criteria | ✅ |
| **P1** | Determinism harness (R=5 repeats) | ✅ |
| **P1** | CI regression gate on golden | local script only |
| **P1** | Separate judge model from worker | ✅ per-skill routing |
| **P1** | Self-consistency on ambiguous soft labels | ✅ optional |
| **P2** | Pairwise for weight calibration only | not used |

---

## Внешние источники

| Источник | Ключевая идея |
|----------|---------------|
| [Braintrust — AI Agent Evaluation](https://www.braintrust.dev/articles/ai-agent-evaluation-framework) | Hybrid deterministic + LLM; regression gates |
| [arXiv:2507.21504 — Agent Eval Survey](https://arxiv.org/html/2507.21504v1) | pass^k consistency; code-based primary |
| [arXiv:2604.02368 — XpertBench / ShotJudge](https://arxiv.org/html/2604.02368v2) | Expert-anchored few-shot; binary checkpoints |
| [Evidently — LLM-as-a-Judge](https://www.evidentlyai.com/llm-guide/llm-as-a-judge) | temp=0; split criteria; rule-first hybrid |

---

## Changelog

### После baseline гибридного скоринга

- Реализованы 11 deterministic tools; hard-критерии больше не зависят от LLM.
- LLM-слой: routing по skill, cascade fallback, optional self-consistency для soft-labels.
- Few-shot exemplars в конфиге промптов; harness для калибровки soft-критериев.
- JSON Schema для fixture; inline fixture в adapter и worker prompt.
- E2E-тест полного цикла задачи с injected fixture.

---

## Связанные документы

- [implementation.md](./implementation.md) — что реализовано сейчас
- [roadmap.md](./roadmap.md) — план доработок
- [llm-as-judge-pattern.md](./llm-as-judge-pattern.md) — паттерны RubricMiddleware
