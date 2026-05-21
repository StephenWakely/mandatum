FROM debian:bookworm-slim

# claude: 224  M
# uv:      49  M

RUN apt-get update && apt-get install -y --no-install-recommends \
      bash \
      ca-certificates \
      curl \
      git \
      gnupg \
      jq \
      ripgrep \
    && rm -rf /var/lib/apt/lists/*

# Claude Code CLI — native binary (no Node.js required)
RUN curl -fsSL https://claude.ai/install.sh | bash

# uv — required by the Speky MCP plugin
RUN curl -LsSf https://astral.sh/uv/install.sh | sh

ENV PATH="/root/.local/bin:${PATH}"

# Default git identity for agent commits (override per-container with
# GIT_AUTHOR_NAME / GIT_AUTHOR_EMAIL / GIT_COMMITTER_* env vars).
RUN git config --global user.name "Mandatum Agent" \
    && git config --global user.email "agent@mandatum.local" \
    && git config --global init.defaultBranch main \
    && git config --global --add safe.directory '*'

WORKDIR /work
