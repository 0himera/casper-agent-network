# Методика оценки

Принципы скоринга для validator-engine. Целевая архитектура: **F3** (см. [roadmap.md](./roadmap.md)).

---

## Базовый принцип

Базовая единица — **измеримые флаги и баллы по рубрике**, а не «лучший ответ». Deterministic checks для формализуемых условий; LLM-as-judge — только для интерпретации, с фиксированными labels → score в коде.

```
input → gates → deterministic checks → rubric scoring → aggregation → pass/fail
```

Pairwise — только для калибровки весов, не как финальный выход.

---

## Целевая архитектура (F3)

```
gate → код-оракул (hard-критерии) → LLM enum-label (soft, temp=0) → код-агрегация + threshold
```

1. **Hard gate + checklist rubric** — формат, sanity, no-leakage
2. **Weighted thresholded rubric** — веса + порог прохождения (≥ 70)
3. **Code verifier + rubric fallback** — числа/правила в коде; LLM → enum labels (`strong` / `partial` / `missing`) для soft-критериев; score считает код

Soft label mapping: `strong` → 100% weight, `partial` → 50%, `missing` → 0%.

Текущее состояние: F3 pipeline реализован (Phase 5); tools — stubs. План доработки: [roadmap.md](./roadmap.md).

---

## Приоритеты внедрения

| P | Механизм | Статус | Фаза |
|---|----------|--------|------|
| **P0** | Real tools as authoritative scorer | stub only | 6 |
| **P0** | `temperature=0` on judge LLM | ✅ | 4 |
| **P0** | Input gates → skip LLM on hard fail | ✅ | 5 |
| **P0** | LLM → enum labels; score in code | ✅ | 5 |
| **P0** | ShotJudge few-shot exemplars | `few_shot: []` | 7 |
| **P1** | Determinism harness (R=5 repeats) | ✅ | 4 |
| **P1** | CI regression gate on golden | local script only | 4 |
| **P1** | Separate judge model from worker | same chain | 8 |
| **P2** | Pairwise for weight calibration only | not used | — |

---

## Внешние источники

| Источник | Ключевая идея |
|----------|---------------|
| [Braintrust — AI Agent Evaluation](https://www.braintrust.dev/articles/ai-agent-evaluation-framework) | Hybrid deterministic + LLM; regression gates |
| [arXiv:2507.21504 — Agent Eval Survey](https://arxiv.org/html/2507.21504v1) | pass^k consistency; code-based primary |
| [arXiv:2604.02368 — XpertBench / ShotJudge](https://arxiv.org/html/2604.02368v2) | Expert-anchored few-shot; binary checkpoints |
| [Evidently — LLM-as-a-Judge](https://www.evidentlyai.com/llm-guide/llm-as-a-judge) | temp=0; split criteria; rule-first hybrid |

---

## Связанные документы

- [implementation.md](./implementation.md) — что реализовано сейчас
- [roadmap.md](./roadmap.md) — фазы доработки до F3
- [llm-as-judge-pattern.md](./llm-as-judge-pattern.md) — паттерны RubricMiddleware
