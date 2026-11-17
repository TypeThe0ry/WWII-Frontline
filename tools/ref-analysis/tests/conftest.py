import shutil

import pytest


def pytest_collection_modifyitems(config, items):
    if shutil.which("ffmpeg") is None:
        skip = pytest.mark.skip(reason="ffmpeg not installed")
        for it in items:
            if "ffmpeg" in it.keywords:
                it.add_marker(skip)
