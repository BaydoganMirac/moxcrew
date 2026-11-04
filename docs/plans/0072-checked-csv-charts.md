# Task0072: Checked CSV charts

Status: Complete; locally verified

Updated: 2026-11-04

Authorization: The owner asked to finish the remaining non-Windows feature gaps.
This card advances T3 locally without a provider/model call, account action or a
new installed dependency.

## Problem

Moxcrew could clean/analyze CSV and create a manually entered interactive chart,
but T3 promised a checked chart and concise report directly from a supplied
dataset. There was no typed operation binding a frozen CSV, exact columns,
aggregation, plotted values and report to the same result.

## Scope

1. Add data.chart for one frozen CSV, an exact category/value column pair and
   sum or average aggregation.
2. Validate rectangular UTF-8 CSV, unique headers, finite numeric values, row and
   category bounds before publishing.
3. Produce a static standalone SVG, chart-data.json and concise report.md from
   the same ordered aggregate data.
4. Escape owner cells/titles in SVG and Markdown; include no script, foreign
   object, link or source path.
5. Add a Turkish/English native form that stages the exact chart request in Chat
   without starting work.

## Non-goals

- No inferred columns, silent row/category truncation, forecasting, correlation
  claim, dashboard hosting or model-written interpretation.
- No arbitrary SVG/HTML supplied by the caller and no external chart library.
- No Windows-native result-access claim before Windows-PC verification.

## Acceptance

- Deterministic coverage proves aggregation, row/category counts, escaping,
  static SVG structure, report values and non-numeric rejection.
- The 800×560 native form stages the exact frozen CSV, columns and aggregation
  but emits no Host command.
- Full workspace, strict Clippy, formatting and diff checks pass, with a retained
  native visual.
