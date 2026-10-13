"""JSON transport for the private native calculation boundary."""

import json
from . import _core


def call(operation, **values):
    return json.loads(_core.invoke(operation, json.dumps(values)))


def analyze_frames(images, classic=False):
    return json.loads(_core.analyze_frames([(image.width, image.height, image.tobytes()) for image in images], classic))
