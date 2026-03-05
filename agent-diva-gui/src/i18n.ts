import { createI18n } from 'vue-i18n';
import zh from './locales/zh';
import en from './locales/en';

const savedLocale = localStorage.getItem('agent-diva-locale') || 'zh';

const zhPatched: any = { ...zh };
zhPatched.settings = {
  ...(zhPatched.settings || {}),
  network: zhPatched.settings?.network || '网络',
};
zhPatched.dashboard = {
  ...(zhPatched.dashboard || {}),
  network: zhPatched.dashboard?.network || '网络',
  networkDesc: zhPatched.dashboard?.networkDesc || '配置 Web 搜索与抓取工具',
};
zhPatched.network = {
  ...(zhPatched.network || {}),
  title: zhPatched.network?.title || '网络工具',
  desc: zhPatched.network?.desc || '配置 Web 搜索 / 抓取运行参数',
  provider: zhPatched.network?.provider || '搜索提供商',
  apiKey: zhPatched.network?.apiKey || 'Brave API Key',
  apiKeyPlaceholder: zhPatched.network?.apiKeyPlaceholder || '输入 Brave API Key...',
  maxResults: zhPatched.network?.maxResults || '最大结果数',
  enableSearch: zhPatched.network?.enableSearch || '启用 Web Search',
  enableFetch: zhPatched.network?.enableFetch || '启用 Web Fetch',
};
zhPatched.cron = {
  ...(zhPatched.cron || {}),
  title: zhPatched.cron?.title || '定时任务管理',
  subtitle: zhPatched.cron?.subtitle || '查看当前运行任务并管理任务生命周期',
  refresh: zhPatched.cron?.refresh || '刷新',
  newTask: zhPatched.cron?.newTask || '新建任务',
  editTask: zhPatched.cron?.editTask || '编辑任务',
  totalTasks: zhPatched.cron?.totalTasks || '任务总数',
  runningTasks: zhPatched.cron?.runningTasks || '运行中',
  pausedTasks: zhPatched.cron?.pausedTasks || '已暂停',
  failedTasks: zhPatched.cron?.failedTasks || '失败任务',
  runningSection: zhPatched.cron?.runningSection || '正在运行',
  allSection: zhPatched.cron?.allSection || '全部任务',
  loading: zhPatched.cron?.loading || '正在加载定时任务...',
  emptyState: zhPatched.cron?.emptyState || '暂无定时任务。',
  noRunningTasks: zhPatched.cron?.noRunningTasks || '当前没有运行中的任务。',
  nextRun: zhPatched.cron?.nextRun || '下次运行',
  lastRun: zhPatched.cron?.lastRun || '上次运行',
  lastStatus: zhPatched.cron?.lastStatus || '上次状态',
  detailTitle: zhPatched.cron?.detailTitle || '任务详情',
  emptyMessage: zhPatched.cron?.emptyMessage || '暂无任务内容。',
  channel: zhPatched.cron?.channel || '频道',
  recipient: zhPatched.cron?.recipient || '接收方',
  pause: zhPatched.cron?.pause || '暂停',
  enable: zhPatched.cron?.enable || '开启',
  runNow: zhPatched.cron?.runNow || '立即执行',
  stop: zhPatched.cron?.stop || '停止',
  edit: zhPatched.cron?.edit || '编辑',
  delete: zhPatched.cron?.delete || '删除',
  cancel: zhPatched.cron?.cancel || '取消',
  save: zhPatched.cron?.save || '保存',
  saving: zhPatched.cron?.saving || '保存中...',
  taskName: zhPatched.cron?.taskName || '任务名称',
  scheduleType: zhPatched.cron?.scheduleType || '调度类型',
  runAt: zhPatched.cron?.runAt || '执行时间',
  everySeconds: zhPatched.cron?.everySeconds || '固定间隔（秒）',
  cronExpr: zhPatched.cron?.cronExpr || 'Cron 表达式',
  timezone: zhPatched.cron?.timezone || '时区',
  message: zhPatched.cron?.message || '任务消息',
  deliver: zhPatched.cron?.deliver || '投递结果',
  enabled: zhPatched.cron?.enabled || '启用',
  deleteAfterRun: zhPatched.cron?.deleteAfterRun || '成功执行后删除',
  notScheduled: zhPatched.cron?.notScheduled || '未计划',
  notRunning: zhPatched.cron?.notRunning || '未运行',
  runningForSeconds: zhPatched.cron?.runningForSeconds || '已运行 {count} 秒',
  runningForMinutes: zhPatched.cron?.runningForMinutes || '已运行 {count} 分钟',
  scheduleAt: zhPatched.cron?.scheduleAt || '单次执行',
  scheduleEverySeconds: zhPatched.cron?.scheduleEverySeconds || '每 {count} 秒',
  confirmDelete: zhPatched.cron?.confirmDelete || '确认删除任务“{name}”？',
  confirmDeleteRunning:
    zhPatched.cron?.confirmDeleteRunning || '任务“{name}”正在运行，将先停止再删除，是否继续？',
  schedule: {
    ...(zhPatched.cron?.schedule || {}),
    at: zhPatched.cron?.schedule?.at || '单次',
    every: zhPatched.cron?.schedule?.every || '间隔',
    cron: zhPatched.cron?.schedule?.cron || 'Cron',
  },
  status: {
    ...(zhPatched.cron?.status || {}),
    running: zhPatched.cron?.status?.running || '运行中',
    scheduled: zhPatched.cron?.status?.scheduled || '已计划',
    paused: zhPatched.cron?.status?.paused || '已暂停',
    completed: zhPatched.cron?.status?.completed || '已完成',
    failed: zhPatched.cron?.status?.failed || '失败',
  },
};

const i18n = createI18n({
  legacy: false,
  locale: savedLocale,
  fallbackLocale: 'en',
  messages: {
    zh: zhPatched,
    en,
  },
});

export default i18n;
