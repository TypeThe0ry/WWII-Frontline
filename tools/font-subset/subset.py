#!/usr/bin/env python3
"""Build verified, self-hosted UI fonts. See tools/asset-pipeline/docs/font-subsetting.md."""
import argparse
import io
import json
import re
import tempfile
from dataclasses import dataclass
from itertools import groupby
from pathlib import Path

import brotli
from fontTools import subset
from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont

from ledger import digest, font_rows, verified_input

ROOT = Path(__file__).resolve().parents[2]
LATIN = set(range(0x20, 0x7F)) | set(range(0xA0, 0x100)) | set(map(ord, '–—‘’“”•…←→−⚠'))
PUNCT = set(map(ord, '、。，：；！？（）—…·《》“”‘’'))


@dataclass(frozen=True)
class Face:
    policy_id: str
    filename: str
    family: str
    weight: str
    axes: dict | None = None
    rfn: str = ''


FACES = (
    Face('AST-V-FNT-UI-01', 'BarlowSemiCondensed-Medium.ttf', 'Barlow Semi Condensed', '500'),
    Face('AST-V-FNT-UI-01', 'BarlowSemiCondensed-SemiBold.ttf', 'Barlow Semi Condensed', '600'),
    Face('AST-V-FNT-004-01', 'IBMPlexSans[wdth,wght].ttf', 'Frontline UI Sans', '400 600', {'wdth': 100, 'wght': (400, 600)}, 'Plex'),
    Face('AST-V-FNT-004-02', 'IBMPlexMono-Medium.ttf', 'Frontline UI Mono', '500', rfn='Plex'),
    Face('AST-V-FNT-004-03', 'NotoSansSC[wght].ttf', 'Noto Sans SC', '400 700', {'wght': (400, 700)}, 'Source'),
)
NOTO = FACES[-1]


def vocabulary(directory):
    boot, game, hashes = set(), set(), {}
    for name in ('en.json', 'zh-Hans.json'):
        data = (directory / name).read_bytes()
        hashes[name] = digest(data)
        for key, text in json.loads(data).items():
            chars = {ord(c) for c in text if c.isprintable()}
            game.update(chars)
            if key.startswith('ui.boot.') or key == 'ui.menu.retry':
                boot.update(chars)
    return boot, game, hashes


def gb_level(start, end):
    return {ord(c) for hi in range(start, end) for lo in range(0xA1, 0xFF)
            for c in bytes((hi, lo)).decode('gb2312', errors='ignore')}


def cjk_shards(boot, game):
    levels = [boot - LATIN | boot & PUNCT, (game - LATIN) | PUNCT,
              gb_level(0xB0, 0xD8), gb_level(0xD8, 0xF8)]
    used, shards = set(), {}
    for i, chars in enumerate(levels):
        shards[f'cjk{i}'] = chars - used
        used.update(chars)
    return shards


def ranges(chars):
    parts = []
    for _, group in groupby(enumerate(sorted(chars)), key=lambda pair: pair[1] - pair[0]):
        values = [v for _, v in group]
        parts.append(f'U+{values[0]:X}' + (f'-{values[-1]:X}' if len(values) > 1 else ''))
    return ', '.join(parts)


def names(font, face):
    for record in font['name'].names:
        text = record.toUnicode()
        if not face.rfn or face.rfn.lower() not in text.lower():
            continue
        if face.rfn == 'Source' and record.nameID == 0:
            # Preserve Adobe's copyright; the verbatim RFN declaration ships in the OFL sidecar.
            text = re.sub(r", with Reserved Font Name 'Source'\.?$", '.', text)
        elif face.rfn == 'Plex' and record.nameID not in (0, 13, 14):
            for old in ('IBM Plex Sans', 'IBM Plex Mono', 'IBMPlexSans', 'IBMPlexMono'):
                text = text.replace(old, face.family if ' ' in old else face.family.replace(' ', ''))
        record.string = text.encode(record.getEncoding())


def write_shard(blob, face, chars, shard, out):
    # Subset first: avoid instancing thousands of glyphs that the shard never uses.
    font = TTFont(io.BytesIO(blob), recalcTimestamp=False)
    actual = chars & font.getBestCmap().keys()
    if not actual or (face == NOTO and actual != chars):
        raise ValueError(f'{shard}: empty or missing required codepoints: {ranges(chars - actual)}')
    opt = subset.Options()
    opt.name_IDs = [0, 1, 2, 3, 4, 5, 6, 13, 14, 16, 17, 25]
    opt.layout_features = ['kern', 'liga', 'locl', 'mark', 'mkmk', 'tnum', 'ccmp']
    opt.notdef_outline, opt.hinting = True, False
    cutter = subset.Subsetter(opt)
    cutter.populate(unicodes=actual)
    cutter.subset(font)
    if face.axes:
        font = instantiateVariableFont(font, face.axes, inplace=True, updateFontNames=True)
    names(font, face)
    font.flavor = 'woff2'
    filename = '-'.join(f'{face.family} {face.weight} {shard}'.lower().split()) + '.woff2'
    path = out / filename
    font.save(path)
    with TTFont(path) as saved:
        if saved.flavor != 'woff2' or saved.getBestCmap().keys() != actual:
            raise ValueError(f'{filename}: serialized cmap mismatch')
        for record in saved['name'].names:
            # Plex copyright and license records are legal notices, not family names.
            if face.rfn == 'Plex' and record.nameID in (0, 13, 14):
                continue
            if face.rfn and face.rfn.lower() in record.toUnicode().lower():
                raise ValueError(f'{filename}: reserved name in name ID {record.nameID}')
    data = path.read_bytes()
    entry = {'file': filename, 'shard': shard, 'bytes': len(data), 'br_bytes': len(brotli.compress(data, quality=11)), 'chars': len(actual)}
    css = (f"@font-face {{ font-family: '{face.family}'; font-weight: {face.weight}; font-style: normal; font-display: swap;\n"
           f"  src: url('./{filename}') format('woff2'); unicode-range: {ranges(actual)}; }}")
    return entry, css


def build(src, out, i18n, diagnostic):
    if out.exists():
        raise ValueError(f'{out}: output must be a new directory (no stale fonts)')
    if diagnostic and out.resolve().is_relative_to(ROOT):
        raise ValueError('use a scratch directory outside the worktree for --check-noto')
    rows = font_rows((ROOT / 'LICENSES.md').read_text())
    inputs, errors = [], []
    for face in ((NOTO,) if diagnostic else FACES):
        try:
            inputs.append((face, *verified_input(ROOT, rows, face, src)))
        except (KeyError, OSError, ValueError) as exc:
            errors.append(f'{face.filename}: {exc}')
    if errors:
        raise ValueError('\n'.join(errors))
    boot, game, hashes = vocabulary(i18n)
    shards = cjk_shards(boot, game)
    report = {'mode': 'noto-check' if diagnostic else 'full', 'catalog_sha256': hashes, 'inputs': [], 'files': []}
    css = []
    out.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=out.parent) as tmp:
        stage = Path(tmp) / 'fonts'
        stage.mkdir()
        for face, blob, asset_id, license_name, notice in inputs:
            (stage / license_name).write_bytes(notice)
            report['inputs'].append({'file': face.filename, 'asset_id': asset_id, 'sha256': digest(blob), 'license': license_name, 'license_sha256': digest(notice)})
            for shard, chars in (shards if face == NOTO else {'latin': LATIN | (game & set(range(0x250)))}).items():
                entry, rule = write_shard(blob, face, chars, shard, stage)
                report['files'].append({**entry, 'asset_id': asset_id})
                css.append(rule)
        latin = sum(f['br_bytes'] for f in report['files'] if f['shard'] == 'latin')
        zh = latin + sum(f['br_bytes'] for f in report['files'] if f['shard'] == 'cjk0')
        report['first_screen_br_bytes'] = None if diagnostic else {'en': latin, 'zh-Hans': zh}
        if not diagnostic and (latin > 204800 or zh > 409600):
            raise ValueError(f'first-screen budget exceeded: en={latin}, zh-Hans={zh} (Brotli bytes)')
        if vocabulary(i18n)[2] != hashes:
            raise ValueError('catalogs changed during build; rerun with current vocabulary')
        (stage / 'fonts.css').write_text('\n'.join(css) + '\n')
        (stage / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
        stage.rename(out)
    return report


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--src', required=True, type=Path, help='local directory of registered TTFs (no download)')
    ap.add_argument('--out', required=True, type=Path)
    ap.add_argument('--i18n', type=Path, default=ROOT / 'client/src/ui/i18n')
    ap.add_argument('--check-noto', action='store_true', help='Noto-only diagnostic; not a release budget check')
    a = ap.parse_args()
    try:
        print(json.dumps(build(a.src, a.out, a.i18n, a.check_noto), indent=2))
    except (OSError, ValueError) as exc:
        ap.exit(2, f'{exc}\n')


if __name__ == '__main__':
    main()
