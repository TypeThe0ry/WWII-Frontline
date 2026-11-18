#!/usr/bin/env python3
"""Font subsetting (M1-065a, UI_SPEC §2.3): Latin + CJK shards as WOFF2, vocabulary extracted from the i18n catalogs.

Inputs  : SRC dir with the upstream TTFs (see FONTS below; all SIL OFL 1.1, ledger rows AST-V-FNT-*).
Outputs : OUT dir with `*.woff2` + `fonts.css` (@font-face with unicode-range, font-display: swap) + `report.json`.
Usage   : python3 tools/font-subset/subset.py --src /tmp/fonts --out client/public/fonts
Needs   : pip install fonttools brotli
"""
import argparse, io, json, os, sys
from fontTools import subset
from fontTools.ttLib import TTFont
from fontTools.varLib import instancer

LATIN = list(range(0x20, 0x7F)) + list(range(0xA0, 0x100)) + [0x2013, 0x2014, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2026, 0x2190, 0x2192, 0x2212, 0x00D7, 0x26A0]
CJK_PUNCT = [0x3001, 0x3002, 0xFF0C, 0xFF1A, 0xFF1B, 0xFF01, 0xFF1F, 0xFF08, 0xFF09, 0x2014, 0x2026, 0x00B7, 0x300A, 0x300B, 0x201C, 0x201D, 0x2018, 0x2019]

# family -> (file, axis limits or None, [css weight range])
FONTS = {
    'Barlow Semi Condensed 500': ('BarlowSemiCondensed-Medium.ttf', None, 'Barlow Semi Condensed', '500'),
    'Barlow Semi Condensed 600': ('BarlowSemiCondensed-SemiBold.ttf', None, 'Barlow Semi Condensed', '600'),
    'IBM Plex Sans': ('IBMPlexSans[wdth,wght].ttf', {'wdth': 100, 'wght': (400, 600)}, 'IBM Plex Sans', '400 600'),
    'IBM Plex Mono 500': ('IBMPlexMono-Medium.ttf', None, 'IBM Plex Mono', '500'),
    'Noto Sans SC': ('NotoSansSC[wght].ttf', {'wght': (400, 700)}, 'Noto Sans SC', '400 700'),
}


def gb2312_level1():
    out = []
    for hi in range(0xB0, 0xD8):
        for lo in range(0xA1, 0xFF):
            try:
                out.append(ord(bytes([hi, lo]).decode('gb2312')))
            except UnicodeDecodeError:
                pass
    return out


def vocab(i18n_dir):
    chars = set()
    for f in ('en.json', 'zh-Hans.json'):
        for v in json.load(open(os.path.join(i18n_dir, f), encoding='utf8')).values():
            chars.update(ord(c) for c in v)
    return chars


_instanced = {}


def load(font, axes):
    """Instance a variable font once per file (slow for Noto Sans SC) and return the serialised bytes."""
    if font not in _instanced:
        f = TTFont(font)
        if axes and 'fvar' in f:
            f = instancer.instantiateVariableFont(f, axes)
        buf = io.BytesIO(); f.save(buf)  # re-loaded per shard: avoids stale glyph-order state after instancing
        _instanced[font] = buf.getvalue()
    return TTFont(io.BytesIO(_instanced[font]))


def run(font, codepoints, out, axes):
    f = load(font, axes)
    opt = subset.Options()
    opt.flavor = 'woff2'
    opt.layout_features = ['kern', 'liga', 'locl', 'mark', 'mkmk', 'tnum', 'ccmp']
    opt.name_IDs = [0, 1, 2, 3, 4, 6, 13, 14]  # keep copyright/licence strings (OFL)
    opt.notdef_outline = True
    opt.hinting = False
    s = subset.Subsetter(opt)
    s.populate(unicodes=sorted(set(codepoints)))
    s.subset(f)
    f.flavor = 'woff2'
    f.save(out)
    return os.path.getsize(out)


def ranges(cps):
    cps = sorted(set(cps))
    out, start, prev = [], None, None
    for c in cps:
        if start is None:
            start = prev = c
        elif c == prev + 1:
            prev = c
        else:
            out.append((start, prev)); start = prev = c
    if start is not None:
        out.append((start, prev))
    return ', '.join(f'U+{a:X}' if a == b else f'U+{a:X}-{b:X}' for a, b in out)


UPSTREAM = 'https://raw.githubusercontent.com/google/fonts/main/ofl/'
URLS = {
    'BarlowSemiCondensed-Medium.ttf': 'barlowsemicondensed/BarlowSemiCondensed-Medium.ttf',
    'BarlowSemiCondensed-SemiBold.ttf': 'barlowsemicondensed/BarlowSemiCondensed-SemiBold.ttf',
    'IBMPlexSans[wdth,wght].ttf': 'ibmplexsans/IBMPlexSans%5Bwdth%2Cwght%5D.ttf',
    'IBMPlexMono-Medium.ttf': 'ibmplexmono/IBMPlexMono-Medium.ttf',
    'NotoSansSC[wght].ttf': 'notosanssc/NotoSansSC%5Bwght%5D.ttf',
}


def fetch(src):
    import urllib.request
    os.makedirs(src, exist_ok=True)
    for name, url in URLS.items():
        dst = os.path.join(src, name)
        if not os.path.exists(dst):
            urllib.request.urlretrieve(UPSTREAM + url, dst)
            print('fetched', name, os.path.getsize(dst))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--src', required=True)
    ap.add_argument('--out', required=True)
    ap.add_argument('--i18n', default='client/src/ui/i18n')
    ap.add_argument('--shards', type=int, default=3)
    ap.add_argument('--fetch', action='store_true', help='download the upstream TTFs into --src first')
    a = ap.parse_args()
    if a.fetch:
        fetch(a.src)
    os.makedirs(a.out, exist_ok=True)
    v = vocab(a.i18n)
    first = sorted({c for c in v if c >= 0x2E80} | set(CJK_PUNCT))  # CJK chars the UI uses today (first-screen shard)
    rest = [c for c in gb2312_level1() if c not in set(first)]
    per = -(-len(rest) // a.shards)
    cjk_shards = [('cjk0', first)] + [(f'cjk{i + 1}', rest[i * per:(i + 1) * per]) for i in range(a.shards)]
    css, report = [], {'files': [], 'cjk_first_screen_chars': len(first), 'cjk_total_chars': len(first) + len(rest)}
    for key, (file, axes, fam, weight) in FONTS.items():
        src = os.path.join(a.src, file)
        if not os.path.exists(src):
            print('missing', src, file=sys.stderr); sys.exit(2)
        slug = fam.lower().replace(' ', '-') + '-' + weight.replace(' ', '-')
        sets = cjk_shards if fam == 'Noto Sans SC' else [('latin', LATIN)]
        for name, cps in sets:
            out = os.path.join(a.out, f'{slug}-{name}.woff2')
            size = run(src, cps, out, axes)
            report['files'].append({'file': os.path.basename(out), 'bytes': size, 'chars': len(cps)})
            css.append(f"@font-face {{ font-family: '{fam}'; font-weight: {weight}; font-style: normal; font-display: swap;\n  src: url('./{os.path.basename(out)}') format('woff2'); unicode-range: {ranges(cps)}; }}")
    open(os.path.join(a.out, 'fonts.css'), 'w').write('/* generated by tools/font-subset/subset.py — do not edit */\n' + '\n'.join(css) + '\n')
    json.dump(report, open(os.path.join(a.out, 'report.json'), 'w'), indent=2)
    print(json.dumps({'total_bytes': sum(f['bytes'] for f in report['files']), **{k: report[k] for k in ('cjk_first_screen_chars', 'cjk_total_chars')}}))


if __name__ == '__main__':
    main()
