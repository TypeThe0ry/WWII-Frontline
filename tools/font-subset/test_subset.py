import json
import os
import re
import tempfile
import unittest
from pathlib import Path

import brotli
from fontTools.ttLib import TTFont, newTable

from ledger import approved, digest, font_rows, verified_input
from subset import FACES, LATIN, NOTO, ROOT, build, cjk_shards, gb_level, names, vocabulary


class PipelineTests(unittest.TestCase):
    def test_levels_and_boot_partition(self):
        first, second = gb_level(0xB0, 0xD8), gb_level(0xD8, 0xF8)
        self.assertEqual((len(first), len(second)), (3755, 3008))
        boot, game = set(map(ord, '前线…')), set(map(ord, '前线…龘。'))
        parts = cjk_shards(boot, game)
        self.assertEqual(parts['cjk0'], boot)
        self.assertIn(ord('龘'), parts['cjk1'])
        union = set.union(*parts.values())
        self.assertTrue((first | second | game) <= union)
        self.assertEqual(len(union), sum(map(len, parts.values())))

    def test_catalog_vocabulary_changes(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp)
            (path / 'en.json').write_text('{}')
            catalog = path / 'zh-Hans.json'
            catalog.write_text('{"ui.boot.new": "前线", "ui.menu.retry": "重", "other": "龘"}')
            boot, game, hashes = vocabulary(path)
            self.assertEqual(boot, set(map(ord, '前线重')))
            self.assertEqual(game - boot, {ord('龘')})
            catalog.write_text('{}')
            self.assertNotEqual(vocabulary(path)[2], hashes)

    def test_ledger_checks_status_codes_and_duplicates(self):
        row = {'asset_id': 'AST-V-FNT-X', 'type': 'font', 'license_name_version': 'SIL OFL 1.1',
               'allowed_processing': 'CONVERT, SUBSET', 'status': 'OK', 'license_text_path': 'x'}
        approved(row, {'SUBSET'})
        with self.assertRaisesRegex(ValueError, 'RENAME'):
            approved(row, {'RENAME'})
        with self.assertRaisesRegex(ValueError, 'PENDING'):
            approved({**row, 'status': 'PENDING'}, {'SUBSET'})
        line = '| ' + ' | '.join(row.values()) + ' |'
        table = '| ' + ' | '.join(row) + ' |\n' + line
        self.assertEqual(font_rows(table), {row['asset_id']: row})
        with self.assertRaisesRegex(ValueError, 'duplicate'):
            font_rows(table + '\n' + line)

    def test_bad_font_hash_and_no_partial_output(self):
        with tempfile.TemporaryDirectory() as tmp:
            src = Path(tmp)
            (src / NOTO.filename).write_bytes(b'not registered')
            out = src / 'out'
            with self.assertRaisesRegex(ValueError, 'SHA-256 differs'):
                build(src, out, ROOT / 'client/src/ui/i18n', True)
            self.assertFalse(out.exists())
            out.mkdir()
            with self.assertRaisesRegex(ValueError, 'new directory'):
                build(src, out, src, True)

    def test_missing_or_wrong_license_is_rejected(self):
        rows = font_rows((ROOT / 'LICENSES.md').read_text())
        with tempfile.TemporaryDirectory() as tmp:
            src = Path(tmp)
            blob = b'test-fixture'
            (src / NOTO.filename).write_bytes(blob)
            rows['AST-V-FNT-004-04']['notes'] = 'SHA-256=' + digest(blob)
            with self.assertRaises(FileNotFoundError):
                verified_input(src, rows, NOTO, src)
            lic = src / rows[NOTO.policy_id]['license_text_path']
            lic.parent.mkdir(parents=True)
            lic.write_bytes(b'not the OFL')
            with self.assertRaisesRegex(ValueError, 'notice'):
                verified_input(src, rows, NOTO, src)
            lic.write_bytes(b'Copyright altered\nSIL OPEN FONT LICENSE Version 1.1')
            with self.assertRaisesRegex(ValueError, 'OFL SHA-256'):
                verified_input(src, rows, NOTO, src)

    def test_plex_rename_preserves_copyright(self):
        font = TTFont()
        font['name'] = newTable('name')
        for name_id, text in [(0, 'Copyright IBM; Reserved Font Name Plex'), (1, 'IBM Plex Sans'), (6, 'IBMPlexSans-Medium')]:
            font['name'].setName(text, name_id, 3, 1, 0x409)
        names(font, FACES[2])
        self.assertEqual([r.toUnicode() for r in font['name'].names],
                         ['Copyright IBM; Reserved Font Name Plex', 'Frontline UI Sans', 'FrontlineUISans-Medium'])

    @unittest.skipUnless(os.getenv('FONT_CHECK_DIR') or os.getenv('NOTO_CHECK_DIR'), 'set FONT_CHECK_DIR to check actual WOFF2 output')
    def test_actual_bundle(self):
        full = bool(os.getenv('FONT_CHECK_DIR'))
        out = Path(os.environ['FONT_CHECK_DIR' if full else 'NOTO_CHECK_DIR'])
        report = json.loads((out / 'report.json').read_text())
        boot, game, hashes = vocabulary(ROOT / 'client/src/ui/i18n')
        self.assertEqual(report['catalog_sha256'], hashes, 'catalogs changed; regenerate the scratch bundle')
        self.assertEqual(report['mode'], 'full' if full else 'noto-check')
        faces = FACES if full else (NOTO,)
        rows = font_rows((ROOT / 'LICENSES.md').read_text())
        by_asset = {}
        self.assertCountEqual([r['file'] for r in report['inputs']], [f.filename for f in faces])
        for record in report['inputs']:
            row = rows[record['asset_id']]
            face, = [f for f in faces if f.filename == record['file']]
            by_asset[row['asset_id']] = face
            self.assertEqual(Path(row['raw_path']).name, face.filename)
            self.assertIn('SHA-256=' + record['sha256'] + ' ', row['notes'])
            notice = (out / record['license']).read_bytes()
            self.assertEqual(notice, (ROOT / row['license_text_path']).read_bytes())
            self.assertEqual(record['license_sha256'], digest(notice))
        self.assertCountEqual([p.name for p in out.iterdir()],
                              ['fonts.css', 'report.json', *{r['license'] for r in report['inputs']},
                               *(f['file'] for f in report['files'])])
        css = (out / 'fonts.css').read_text()
        self.assertNotIn('plex', css.lower())
        seen, latin = set(), set()
        br = {'latin': 0, 'cjk0': 0}
        self.assertCountEqual([f['shard'] for f in report['files']], ['latin'] * (4 if full else 0) + ['cjk0', 'cjk1', 'cjk2', 'cjk3'])
        self.assertCountEqual([f['asset_id'] for f in report['files']],
                              [aid for aid, face in by_asset.items() for _ in range(4 if face == NOTO else 1)])
        for entry in report['files']:
            path = out / entry['file']
            face = by_asset[entry['asset_id']]
            self.assertEqual(entry['bytes'], path.stat().st_size)
            self.assertEqual(entry['br_bytes'], len(brotli.compress(path.read_bytes(), quality=11)))
            if entry['shard'] in br:
                br[entry['shard']] += entry['br_bytes']
            with TTFont(path) as font:
                self.assertEqual(font.flavor, 'woff2')
                if face.axes:
                    axis, = font['fvar'].axes
                    self.assertEqual((axis.axisTag, axis.minValue, axis.maxValue), ('wght', *map(int, face.weight.split())))
                else:
                    self.assertNotIn('fvar', font)
                    self.assertEqual(font['OS/2'].usWeightClass, int(face.weight))
                self.assertEqual(font['name'].getBestFamilyName(), face.family)
                if face.rfn:
                    self.assertFalse(any(face.rfn.lower() in r.toUnicode().lower() for r in font['name'].names))
                chars = set(font.getBestCmap())
            rule = (f"@font-face {{ font-family: '{face.family}'; font-weight: {face.weight}; font-style: normal; font-display: swap;\n"
                    f"  src: url('./{path.name}') format('woff2'); unicode-range: ")
            match = re.search(re.escape(rule) + r'([^;]+);', css)
            self.assertIsNotNone(match)
            declared = set()
            for span in match[1].split(', '):
                self.assertTrue(span.startswith('U+'))
                values = [int(v, 16) for v in span[2:].split('-')]
                declared.update(range(values[0], values[-1] + 1))
            self.assertEqual(declared, chars)
            self.assertEqual(len(chars), entry['chars'])
            if face != NOTO:
                self.assertEqual(entry['shard'], 'latin')
                latin.update(chars)
                continue
            self.assertTrue(seen.isdisjoint(chars))
            if entry['shard'] == 'cjk0':
                self.assertTrue(boot - LATIN <= chars)
                self.assertTrue((chars - boot).isdisjoint(game))
            seen.update(chars)
        self.assertTrue((gb_level(0xB0, 0xF8) | (game - LATIN)) <= seen)
        if full:
            self.assertTrue(game <= seen | latin)
            zh = sum(br.values())
            self.assertEqual(report['first_screen_br_bytes'], {'en': br['latin'], 'zh-Hans': zh})
            self.assertLessEqual(br['latin'], 204800)
            self.assertLessEqual(zh, 409600)
        else:
            self.assertIsNone(report['first_screen_br_bytes'])
