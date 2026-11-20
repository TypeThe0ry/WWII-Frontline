import { render } from 'preact';
import { useEffect, useState } from 'preact/hooks';
import { Button, Panel, Frame, KeyCap, Icon, ProgressBar, ProgressRing, Gauge, Toast, Banner,
  Skeleton, Modal, Tabs, SegmentedControl, Select, Slider, TextField, Toggle, Tooltip, VirtualList } from './components';
import { locale, setLocale, t } from './i18n';
import './i18n/catalogs';
import { GalleryUiPerf } from './perf/GalleryUiPerf';
import { installUiFonts } from './fonts/loader';

const languages = [{ value: 'en', labelKey: 'gallery.english' }, { value: 'zh-Hans', labelKey: 'gallery.chinese' }];
const groups = [{ id: 'controls', labelKey: 'gallery.controls' }, { id: 'readouts', labelKey: 'gallery.readouts' }];
const entries = Array.from({ length: 1000 }, (_, i) => i + 1);

function Gallery({ checkFonts }: { checkFonts: boolean }) {
  const [tab, setTab] = useState('controls');
  const [mode, setMode] = useState('controls');
  const [level, setLevel] = useState(50);
  const [enabled, setEnabled] = useState(false);
  const [nickname, setNickname] = useState('');
  const [open, setOpen] = useState(false);
  const [confirmed, setConfirmed] = useState(false);
  const language = locale.value;
  useEffect(() => { document.documentElement.lang = language; document.title = t('gallery.title'); }, [language]);
  useEffect(() => { if (checkFonts) return installUiFonts(); }, [checkFonts]);

  const controls = <Panel title={t('gallery.controls')}>
    <TextField labelKey="ui.browser.nickname" value={nickname} onChange={setNickname} />
    <Slider labelKey="gallery.level" min={0} max={100} value={level} onChange={setLevel} />
    <Toggle labelKey="gallery.enabled" value={enabled} onChange={setEnabled} />
    <SegmentedControl label={t('gallery.mode')} items={groups} value={mode} onChange={setMode} />
    <Tooltip contentKey="gallery.confirm"><button type="button" onClick={() => setOpen(true)}>{t('gallery.preview')}</button></Tooltip>
    <Button disabled reason={t('ui.browser.reason.full')}>{t('ui.browser.join')}</Button>
    {confirmed && <Banner notice={{ id: 'confirmed', kind: 'banner', severity: 'ok', i18nKey: 'gallery.ready' }} />}
    {enabled && <Toast notice={{ id: 'preview', kind: 'toast', severity: 'info', i18nKey: 'gallery.notice' }} />}
  </Panel>;
  const readouts = <Panel title={t('gallery.readouts')}>
    <KeyCap action="interact" />
    <svg aria-hidden="true" width="0" height="0"><symbol id="PH_gallery_marker" /></svg>
    <Icon id="PH_gallery_marker" label={t('gallery.frame')} />
    <Frame affiliation="neutral" glyph="PH_gallery_marker" label={t('gallery.frame')} />
    <ProgressBar label={t('gallery.level')} value={level} max={100} />
    <ProgressRing label={t('gallery.level')} value={level} max={100} />
    <Gauge label={t('gallery.level')} value={level} min={0} max={100} ticks={5} />
    <Skeleton shape="row" />
    <VirtualList items={entries} label={t('gallery.entries')} rowHeight={28} height={140}
      renderRow={n => t('gallery.item', { n })} />
  </Panel>;
  return <main data-ui-gallery="">
    <h1>{t('gallery.title')}</h1>
    <Select labelKey="ui.menu.language" options={languages} value={language}
      onChange={value => setLocale(value === 'zh-Hans' ? 'zh-Hans' : 'en')} />
    <Tabs label={t('gallery.pages')} items={groups.map(g => ({ ...g, panel: g.id === 'controls' ? controls : readouts }))}
      value={tab} onChange={setTab} />
    {open && <Modal titleKey="gallery.confirm" onClose={() => setOpen(false)} actions={[
      { labelKey: 'common.cancel', onClick: () => setOpen(false) },
      { labelKey: 'common.ok', onClick: () => { setConfirmed(true); setOpen(false); } },
    ]}>{nickname}</Modal>}
    <GalleryUiPerf />
  </main>;
}

export function mountGallery(root: HTMLElement, params: URLSearchParams) {
  setLocale(params.get('lang') === 'zh-Hans' ? 'zh-Hans' : 'en');
  render(<Gallery checkFonts={params.get('fonts') === '1'} />, root);
}
