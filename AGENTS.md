# AGENTS.md

## Overview

real-time life simulation app with llm. rust app.

## Principles

- mock api keys for test. and keys never appear in debug or errors.
- use `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `HOME`. so real save and keys aren't touched.
- claude schema no `pattern`, no length/count/number limit. request no `thinking`, `temperature`, prefill. api say 400. check in code.
- yo, just dont waste your time for test too much. it's not good for this style app. feel free to overhaul and create anything you want. for visual test, do it when you REALLY need it. meaning you can imagine roughly, just dont waste your time.
- commit and push easily. dont care bout semantic of each commit or branch or whatever. ez ez.

## Conventions

- test inline `#[cfg(test)] mod tests`. name is sentence, snake_case. like `a_rolling_ball_stays_in_the_room_and_comes_to_rest`. test behavior and math, not looks.
- full word names. `columns` not `cols`.
