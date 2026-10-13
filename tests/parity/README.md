# Frozen Python parity reference

Generated before porting from `c55b9bf`, Python 3.14 and Pillow 12.3.0, with
seed 8675309. `golden/manifest.json` summarizes coverage. The lossless PNG files
are the exact normalized RGBA frames, including all four GIF samples. The
compressed JSON contains complete outputs, not hashes or selected color roles.

Generate only against the original Python implementation:

```sh
python tests/parity/make_golden.py
```

Compare parsed outputs through the installed package:

```sh
python tests/parity/check_golden.py -q
```

Only dictionary fields named `engine` and `path` are excluded from comparisons.
For synthetic inputs, the checker supplies the original encoded-source digest
from the reference, since these tests start at the normalized-pixel boundary.
It does not compare the digest of a normalized PNG to the original image file.
Exceptions are compared by type and message. Never regenerate fixtures to accept
a porting discrepancy. Rust tests consume the same reference and PNG frames.

`benchmark.py` runs each gallery image three times in fresh processes and caches.
It measures `from_image` wall time (including decode and cache writes) and Linux
process peak RSS in KiB. Run both implementations with the same Pillow/Python.
