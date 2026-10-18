# Repository Guidelines

## Project Structure & Module Organization

This repository is currently a minimal scaffold for `wave-ascii`. The only tracked project document is `README.md`; no source code, tests, assets, or dependency manifests exist yet.

As implementation begins, group related code into clearly named modules. Introduce directories such as `src/`, `tests/`, or `assets/` only when there is content to organize, and update the README to explain their purpose.

## Build, Test, and Development Commands

No build system, development server, or automated test commands are configured. Do not assume commands such as `npm test` or `make build` are available.

Useful repository checks include:

- `git status --short`: inspect pending changes.
- `git diff --check`: catch whitespace errors in tracked changes.
- `git diff`: review the patch before committing.

When adding tooling, document its installation requirements and exact build, run, and test commands in `README.md`.

## Coding Style & Naming Conventions

No language-specific style, formatter, or linter has been established. Match the conventions of surrounding files and use consistent indentation within each file. Keep Markdown headings descriptive and examples executable.

Choose descriptive filenames and identifiers. When introducing a language or framework, add an appropriate formatting configuration rather than relying on undocumented personal preferences.

## Testing Guidelines

There is no testing framework or coverage threshold yet. When adding executable behavior, include relevant tests and document how to run them. Use descriptive test names that state the behavior being checked, and keep fixtures close to the tests that use them.

## Commit & Pull Request Guidelines

The Git history contains only `Initial commit`, so no recurring commit convention is established. Use concise, imperative commit subjects, such as `Add wave rendering module`.

Keep pull requests focused. Describe the change, its motivation, and validation performed; link related issues when available. Include screenshots or sample terminal output when a change affects visible output. Explicitly state when tests could not be run and why.
