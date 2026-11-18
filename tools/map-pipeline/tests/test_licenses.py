from pathlib import Path

from map_pipeline import carentan, licenses as lic

ROOT = Path(__file__).resolve().parents[3]
HEADER = "| asset_id | status | purchase_date |\n|---|---|---|\n"


def _meta(root=ROOT):
    return carentan.build_meta(root, "")


def test_repo_ledger_state_matches_the_h052_verification():
    ledger = lic.parse_ledger((ROOT / "LICENSES.md").read_text(encoding="utf-8"))
    rep = {r["id"]: r for r in lic.check_sources(carentan.build_meta(ROOT), ledger, ROOT)}
    assert rep["ign_lidar_hd"]["status"] == lic.OK and rep["osm"]["status"] == lic.OK
    # Commercial use unspecified (GLO-30) and official page unread (RGE ALTI): never OK.
    assert rep["copernicus_glo30"]["status"] == lic.PENDING
    assert rep["ign_rge_alti"]["status"] == lic.PENDING
    assert lic.MISSING not in {r["status"] for r in rep.values()}
    assert lic.false_ok(list(rep.values())) == []


def test_example_rows_are_ignored():
    rows = lic.parse_ledger(HEADER + "| AST-EXAMPLE-1 | OK | 2026-01-01 |\n| AST-M-OSM-01 | PENDING | — |\n")
    assert list(rows) == ["AST-M-OSM-01"]


def test_ok_needs_both_ledger_ok_and_archive(tmp_path):
    ledger = lic.parse_ledger(
        HEADER + "| AST-M-OSM-01 | OK | 2026-11-20 |\n| AST-M-LIDAR-01 | PENDING | — |\n")
    meta = _meta(tmp_path)
    rep = {r["id"]: r for r in lic.check_sources(meta, ledger, tmp_path)}
    assert rep["osm"]["status"] == lic.PENDING and "not archived" in " ".join(rep["osm"]["reasons"])
    (tmp_path / "licenses/texts").mkdir(parents=True)
    (tmp_path / "licenses/texts/AST-M-OSM-01.txt").write_text("ODbL")
    rep = {r["id"]: r for r in lic.check_sources(_meta_with_ledger(tmp_path, ledger), ledger, tmp_path)}
    assert rep["osm"]["status"] == lic.OK and rep["osm"]["reasons"] == []
    assert rep["ign_lidar_hd"]["status"] == lic.PENDING
    assert rep["ign_rge_alti"]["status"] == lic.MISSING


def _meta_with_ledger(root, ledger):
    text = "".join(f"| {k} | {v['status']} | {v['purchase_date']} |\n" for k, v in ledger.items())
    return carentan.build_meta(root, HEADER + text)


def test_verified_date_only_from_ok_ledger_row_with_archive(tmp_path):
    ledger = HEADER + "| AST-M-OSM-01 | OK | 2026-11-20 |\n"
    assert {s["id"]: s["verified"] for s in carentan.build_meta(tmp_path, ledger)["sources"]}["osm"] is None
    (tmp_path / "licenses/texts").mkdir(parents=True)
    (tmp_path / "licenses/texts/AST-M-OSM-01.txt").write_text("x")
    got = {s["id"]: s["verified"] for s in carentan.build_meta(tmp_path, ledger)["sources"]}
    assert got["osm"] == "2026-11-20" and got["ign_lidar_hd"] is None


def test_meta_claiming_verified_without_ledger_is_not_ok():
    meta = _meta()
    meta["sources"][0]["verified"] = "2026-11-17"
    rep = lic.check_sources(meta, {}, ROOT)
    assert rep[0]["status"] != lic.OK
    assert lic.false_ok(rep) == []


def test_draft_rows_are_all_pending_and_never_ok():
    text = lic.draft_rows(carentan.SOURCES)
    rows = [ln for ln in text.splitlines() if ln.startswith("| AST-M-")]
    assert len(rows) == 4
    assert all("| PENDING |" in r and "| OK |" not in r for r in rows)
    parsed = lic.parse_ledger(text)
    assert {r["status"] for r in parsed.values()} == {"PENDING"}


def test_license_notice_has_odbl_and_attribution():
    t = (ROOT / "data/maps/carentan/LICENSE_NOTICE.md").read_text(encoding="utf-8")
    assert "ODbL" in t and "© OpenStreetMap contributors" in t
