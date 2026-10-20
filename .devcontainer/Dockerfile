FROM docker.io/library/rust:latest

# run `COPY --from=` here if necessary for multi stg

# here fixed(maybe)
RUN curl \
    -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg \
    -o /etc/apt/keyrings/githubcli-archive-keyring.gpg \
    && echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" > /etc/apt/sources.list.d/github-cli.list \
    && apt-get update \
    && apt-get install -y --no-install-recommends gh git less \
    && rm -rf /var/lib/apt/lists/* \
    # the dev user; uid 1000 matches the usual host user, so files in the mounted repo keep your ownership
    && useradd --create-home --uid 1000 --shell /bin/bash dev

USER dev

ENV PATH="/home/dev/.local/bin:${PATH}"

RUN mkdir -p \
    ~/.claude \
    ~/.codex \
    ~/.gemini \
    ~/.config/gh \
    ~/.config/git \
    && touch ~/.config/git/config

RUN curl -fsSL https://chatgpt.com/codex/install.sh -o /tmp/install-codex.sh \
    && CODEX_NON_INTERACTIVE=1 sh /tmp/install-codex.sh \
    && rm /tmp/install-codex.sh

RUN curl -fsSL https://claude.ai/install.sh -o /tmp/install-claude.sh \
    && bash /tmp/install-claude.sh \
    && rm /tmp/install-claude.sh

RUN curl -fsSL https://antigravity.google/cli/install.sh -o /tmp/install-agy.sh \
    && bash /tmp/install-agy.sh \
    && rm /tmp/install-agy.sh

# here custom
# rust: editor support (rust-analyzer, rust-src), formatter and linter
RUN rustup component add rust-analyzer rust-src rustfmt clippy
