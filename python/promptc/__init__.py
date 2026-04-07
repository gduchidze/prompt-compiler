"""promptc — Python bindings for the prompt compiler engine."""

from __future__ import annotations

__version__ = "0.3.0"

try:
    from .promptc_core import compile as _compile
    from .promptc_core import check_gptisms as _check_gptisms

    def compile(source: str, target: str = "claude", opt_level: int = 2) -> str:
        """Compile a prompt for a target model.

        Args:
            source: The prompt source text.
            target: Target model — "claude", "gpt", "mistral", or "llama".
            opt_level: Optimization level — 0 (none), 1 (safe), 2 (full).

        Returns:
            The compiled prompt string.
        """
        return _compile(source, target, opt_level)

    def check_gptisms(text: str) -> list[tuple[str, str, str]]:
        """Detect GPT-isms in prompt text.

        Args:
            text: The prompt text to check.

        Returns:
            List of (found, suggestion, severity) tuples.
        """
        return _check_gptisms(text)

except ImportError:
    pass
