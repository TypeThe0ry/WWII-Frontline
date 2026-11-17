import pytest

from map_pipeline.cli import generate_m0


@pytest.fixture(scope="session")
def m0_dir(tmp_path_factory):
    out = tmp_path_factory.mktemp("m0")
    generate_m0(out, seed=1)
    return out
