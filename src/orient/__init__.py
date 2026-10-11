"""Orient: portable image-derived UI palettes, independent of desktop publishers."""

import hashlib
from pathlib import Path

__version__ = "0.1.0.dev2"
FORMAT_VERSION = 1
_root = Path(__file__).parent
_hash = hashlib.sha256()
for _name in ("colour.py", "extract.py", "classic.py", "palette.py", "engine.py", "accessibility.py"):
    if (_root / _name).exists():
        _hash.update((_root / _name).read_bytes())
ENGINE_ID = "orient-" + __version__ + "-" + _hash.hexdigest()[:12]
