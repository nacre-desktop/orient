"""Compatibility API for the Rust Natural selection policy."""

from pathlib import Path
from .extract import frames
from ._bridge import analyze_frames


def analyze(path):
    path = Path(path).expanduser().resolve(strict=True)
    return analyze_frames(frames(path), classic=True)
