"""Read the project's font approvals by column name, not by filename alone."""
import hashlib
import re
from pathlib import Path


def digest(data):
    return hashlib.sha256(data).hexdigest()


def font_rows(text):
    rows, header = {}, []
    for line in text.splitlines():
        cells = [c.strip() for c in line.strip().strip('|').split('|')]
        if 'asset_id' in cells and 'license_text_path' in cells:
            header = cells
        elif header and line.startswith('| AST-V-FNT-'):
            row = dict(zip(header, cells, strict=True))
            key = row['asset_id']
            if key in rows:
                raise ValueError(f'duplicate ledger row: {key}')
            rows[key] = row
    return rows


def approved(row, required):
    if row['type'] != 'font' or row['license_name_version'] != 'SIL OFL 1.1' or row['status'] != 'OK':
        raise ValueError(f"{row['asset_id']}: expected an OK OFL font (status {row['status']})")
    missing = required - {s.strip() for s in row['allowed_processing'].split(',')}
    if missing:
        raise ValueError(f"{row['asset_id']}: missing processing approval {sorted(missing)}")


def verified_input(root, rows, spec, src):
    required = {'CONVERT', 'SUBSET'}
    if spec.rfn == 'Plex':
        required.add('RENAME')
    policy = rows[spec.policy_id]
    approved(policy, required)
    matches = [r for r in rows.values() if Path(r['raw_path']).name == spec.filename]
    if len(matches) != 1:
        raise ValueError(f'{spec.filename}: need one ledger raw_path and SHA-256')
    row, = matches
    approved(row, required)
    if row['license_text_path'] != policy['license_text_path']:
        raise ValueError(f'{spec.filename}: source and family OFL notices differ')
    pins = re.findall(r'\bSHA-256=([0-9a-fA-F]{64})\b', row['notes'])
    if len(pins) != 1:
        raise ValueError(f'{spec.filename}: need one ledger SHA-256')
    blob = (src / spec.filename).read_bytes()
    if digest(blob) != pins[0].lower():
        raise ValueError(f'{spec.filename}: SHA-256 differs from {row["asset_id"]}')
    license_path = (root / row['license_text_path']).resolve()
    if not license_path.is_relative_to((root / 'licenses/texts').resolve()):
        raise ValueError(f'{spec.filename}: invalid license_text_path')
    notice = license_path.read_bytes()
    if b'SIL OPEN FONT LICENSE Version 1.1' not in notice or b'Copyright' not in notice:
        raise ValueError(f'{license_path}: missing OFL/copyright notice')
    if re.findall(r'\bOFL_SHA256=([0-9a-f]{64})\b', row['notes']) != [digest(notice)]:
        raise ValueError(f'{license_path}: OFL SHA-256 differs from {row["asset_id"]}')
    return blob, row['asset_id'], license_path.name, notice
