import en from '../i18n/en.json';
import zh from '../i18n/zh-Hans.json';
import type { Locale } from '../i18n';

// Task-local labels; reuse the shared keys for existing pause and binding messages.
const text = {
  en: {
    pause: 'Pause menu', redeploy: 'Redeploy / change role', redeployConfirm: 'Redeploy now?', team: 'Change team',
    report: 'Report player', vote: 'Start vote', squad: 'Squad management', confirm: 'Confirm', cancel: 'Cancel', back: 'Back',
    audio: 'Audio', controls: 'Controls', video: 'Graphics', language: 'Language', diagnostics: 'Network / diagnostics',
    'audio.master': 'Master volume', 'audio.sfx': 'Effects volume', 'audio.ui': 'Interface volume',
    'input.sensH': 'Horizontal sensitivity', 'input.sensV': 'Vertical sensitivity', 'input.adsMult': 'ADS multiplier',
    'input.invertY': 'Invert Y', 'video.fov': 'Field of view', 'quality.tier': 'Quality preset (placeholder)',
    'ui.lang': 'Language', 'net.showStats': 'Show ping, packet loss and FPS', copy: 'Copy diagnostics',
    copied: 'Diagnostics copied.', copyFailed: 'Could not copy diagnostics.',
    low: 'Low', medium: 'Medium', high: 'High', primary: 'Primary', secondary: 'Secondary',
    bindings: 'Key bindings', press: 'Press a new key or mouse button; Escape cancels.', unassigned: 'Unassigned',
    swap: 'Swap', unbind: 'Unbind other action', invalid: 'Invalid or conflicting binding scheme.',
    scheme: 'Binding scheme', save: 'Save scheme', load: 'Load scheme', saved: 'Scheme saved.', loaded: 'Scheme loaded.',
    json: 'Binding JSON', import: 'Import JSON', export: 'Export JSON',
    storage: 'Local storage is unavailable; changes are kept for this session only.', locked: 'Requesting pointer lock…',
  },
  'zh-Hans': {
    pause: '暂停菜单', redeploy: '重新部署 / 换兵种', redeployConfirm: '现在重新部署？', team: '换队',
    report: '举报玩家', vote: '发起投票', squad: '小队管理', confirm: '确认', cancel: '取消', back: '返回',
    audio: '音频', controls: '控制', video: '画质', language: '语言', diagnostics: '网络 / 诊断',
    'audio.master': '主音量', 'audio.sfx': '音效音量', 'audio.ui': '界面音量',
    'input.sensH': '水平灵敏度', 'input.sensV': '垂直灵敏度', 'input.adsMult': '瞄准倍率',
    'input.invertY': '反转 Y', 'video.fov': '视野', 'quality.tier': '画质预设（占位）',
    'ui.lang': '语言', 'net.showStats': '显示延迟、丢包和帧率', copy: '复制诊断信息',
    copied: '已复制诊断信息。', copyFailed: '无法复制诊断信息。',
    low: '低', medium: '中', high: '高', primary: '主键', secondary: '副键',
    bindings: '键位', press: '请按新键或鼠标按钮；Esc 取消。', unassigned: '未绑定',
    swap: '互换', unbind: '解除对方绑定', invalid: '键位方案无效或有冲突。',
    scheme: '键位方案', save: '保存方案', load: '加载方案', saved: '已保存方案。', loaded: '已加载方案。',
    json: '键位 JSON', import: '导入 JSON', export: '导出 JSON',
    storage: '本地存储不可用；更改仅在本次会话有效。', locked: '正在锁定指针…',
  },
} as const;

export type CopyKey = keyof typeof text.en | keyof typeof en;
export function makeCopy(locale: Locale): (key: CopyKey) => string {
  const shared = locale === 'en' ? en : zh;
  return (key) => key in text.en ? text[locale][key as keyof typeof text.en] : shared[key as keyof typeof en];
}
