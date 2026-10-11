"""Portable optional cache storage. No desktop state or publication."""

import hashlib
import json
import os
from pathlib import Path
import tempfile
from . import ENGINE_ID


def roots():
    home = Path.home()
    return tuple(
        Path(os.environ.get(name, str(home / default))) / "orient"
        for name, default in (
            ("XDG_CONFIG_HOME", ".config"),
            ("XDG_STATE_HOME", ".local/state"),
            ("XDG_CACHE_HOME", ".cache"),
        )
    )


def read(path, default):
    try:
        return json.loads(path.read_text())
    except FileNotFoundError:
        return default


def write_json(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, name = tempfile.mkstemp(dir=path.parent)
    try:
        with os.fdopen(fd, "w") as stream:
            json.dump(data, stream, sort_keys=True)
        os.chmod(name, 0o600)
        os.replace(name, path)
    finally:
        Path(name).unlink(missing_ok=True)


def cache_key(path, settings):
    path = Path(path).resolve(strict=True)
    stat = path.stat()
    identity = (str(path), stat.st_dev, stat.st_ino, stat.st_size, stat.st_mtime_ns, stat.st_ctime_ns)
    return hashlib.sha256(json.dumps((ENGINE_ID, identity, settings), sort_keys=True).encode()).hexdigest()
