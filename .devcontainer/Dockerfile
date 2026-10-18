FROM docker.io/library/rust:latest

RUN rustup component add rust-analyzer rust-src rustfmt clippy \
    && useradd --create-home --uid 1000 --shell /bin/bash dev

USER dev

ENV PATH="/home/dev/.local/bin:${PATH}"

RUN curl -fsSL https://chatgpt.com/codex/install.sh -o /tmp/install-codex.sh \
    && CODEX_NON_INTERACTIVE=1 sh /tmp/install-codex.sh \
    && rm /tmp/install-codex.sh

RUN curl -fsSL https://claude.ai/install.sh -o /tmp/install-claude.sh \
    && bash /tmp/install-claude.sh \
    && rm /tmp/install-claude.sh

RUN curl -fsSL https://antigravity.google/cli/install.sh -o /tmp/install-agy.sh \
    && bash /tmp/install-agy.sh \
    && rm /tmp/install-agy.sh
