"""Thin API for the pure Rust renderer, also usable by legacy file loaders."""

# The desktop's legacy loader imports this file under an arbitrary module name.
if not __package__:
    import sys
    from pathlib import Path

    sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from orient._bridge import call


def requested_modes(template):
    return set(call("requested_modes", template=template))


def requested_contexts(template):
    return set(call("requested_contexts", template=template))


def render_values(template, values, schemes=None, image=None, custom=None, data=None):
    return call(
        "render_values", template=template, values=values, schemes=schemes, image=image, custom=custom, data=data
    )
