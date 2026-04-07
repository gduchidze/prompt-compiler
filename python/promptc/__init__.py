"""
promptc — A compiler for LLM prompts.

Usage:
    from promptc import compile, check_gptisms

    # Compile a prompt for Claude
    compiled = compile("## Instructions\\n- Be concise.", target="claude", opt_level=2)

    # Check for GPT-isms
    findings = check_gptisms("Let's think step by step.")
    for found, suggestion, severity in findings:
        print(f"[{severity}] {found} → {suggestion}")
"""

from .promptc import compile, check_gptisms

__all__ = ["compile", "check_gptisms"]
