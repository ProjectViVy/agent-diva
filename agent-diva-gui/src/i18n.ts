import { createI18n } from 'vue-i18n';
import zh from './locales/zh';
import en from './locales/en';
import { LOCALE_STORAGE_KEY } from './utils/localStorageAgentDiva';

const savedLocale = localStorage.getItem(LOCALE_STORAGE_KEY) || 'zh';

const zhPatched: any = { ...zh };

zhPatched.settings = {
  ...(zhPatched.settings || {}),
  network: zhPatched.settings?.network || '网络',
  skills: zhPatched.settings?.skills || '技能',
  mcp: zhPatched.settings?.mcp || 'MCP',
};

zhPatched.dashboard = {
  ...(zhPatched.dashboard || {}),
  skills: zhPatched.dashboard?.skills || '技能',
  skillsDesc: zhPatched.dashboard?.skillsDesc || '上传、查看并删除工作区技能',
  network: zhPatched.dashboard?.network || '网络',
  networkDesc: zhPatched.dashboard?.networkDesc || '配置 Web 搜索与抓取工具',
  mcp: zhPatched.dashboard?.mcp || 'MCP',
  mcpDesc:
    zhPatched.dashboard?.mcpDesc || '管理 MCP 服务、连接状态与当前运行集合',
};

zhPatched.general = {
  ...(zhPatched.general || {}),
  cacheTitle: zhPatched.general?.cacheTitle || '界面缓存',
  cacheDesc:
    zhPatched.general?.cacheDesc ||
    '清除已保存模型、会话缓存和本地显示偏好，不影响配置文件中的 provider 凭据。',
  clearCache: zhPatched.general?.clearCache || '清除缓存',
  cacheCleared: zhPatched.general?.cacheCleared || '缓存已清除',
  skillsTitle: zhPatched.general?.skillsTitle || '技能管理',
  skillsDesc:
    zhPatched.general?.skillsDesc || '上传工作区技能、删除自定义技能，并查看当前可见技能状态。',
  refreshSkills: zhPatched.general?.refreshSkills || '刷新技能',
  uploadSkill: zhPatched.general?.uploadSkill || '上传 ZIP',
  uploadingSkill: zhPatched.general?.uploadingSkill || '上传中...',
  skillsZipHint:
    zhPatched.general?.skillsZipHint || '上传包含 SKILL.md 的 ZIP 技能包，支持附带 scripts 和 assets。',
  skillsPreviewOnly:
    zhPatched.general?.skillsPreviewOnly || '浏览器预览模式仅显示外观，上传和删除需要在 Tauri 中执行。',
  loadingSkills: zhPatched.general?.loadingSkills || '正在加载技能...',
  emptySkills: zhPatched.general?.emptySkills || '当前没有可见技能。',
  skillStatusActive: zhPatched.general?.skillStatusActive || '已启用',
  skillStatusAvailable: zhPatched.general?.skillStatusAvailable || '可用',
  skillStatusUnavailable: zhPatched.general?.skillStatusUnavailable || '不可用',
  skillSourceBuiltin: zhPatched.general?.skillSourceBuiltin || '内置',
  skillSourceWorkspace: zhPatched.general?.skillSourceWorkspace || '工作区',
  deleteSkill: zhPatched.general?.deleteSkill || '删除技能',
  builtinSkillLocked: zhPatched.general?.builtinSkillLocked || '内置锁定',
  skillUnavailableHint:
    zhPatched.general?.skillUnavailableHint || '该技能已被发现，但当前缺少运行依赖，暂不可用。',
  skillsZipOnly: zhPatched.general?.skillsZipOnly || '仅支持上传 .zip 技能包。',
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

zhPatched.mcp = {
  ...(zhPatched.mcp || {}),
  title: zhPatched.mcp?.title || 'MCP 管理',
  desc:
    zhPatched.mcp?.desc || '管理 MCP 服务、探测后端连接状态，并控制当前运行集合',
  refresh: zhPatched.mcp?.refresh || '刷新',
  add: zhPatched.mcp?.add || '新建 MCP',
  importJson: zhPatched.mcp?.importJson || '导入 JSON',
  empty: zhPatched.mcp?.empty || '当前还没有配置 MCP 服务。',
  activeTitle: zhPatched.mcp?.activeTitle || '当前使用',
  activeDesc: zhPatched.mcp?.activeDesc || '已启用并已应用到运行时的 MCP 集合',
  total: zhPatched.mcp?.total || '总数',
  online: zhPatched.mcp?.online || '在线',
  degraded: zhPatched.mcp?.degraded || '异常',
  disabled: zhPatched.mcp?.disabled || '已禁用',
  formCreate: zhPatched.mcp?.formCreate || '新建 MCP',
  formEdit: zhPatched.mcp?.formEdit || '编辑 MCP',
  save: zhPatched.mcp?.save || '保存 MCP',
  saving: zhPatched.mcp?.saving || '保存中...',
  cancel: zhPatched.mcp?.cancel || '取消',
  name: zhPatched.mcp?.name || '名称',
  transport: zhPatched.mcp?.transport || '连接方式',
  transportStdio: zhPatched.mcp?.transportStdio || 'Stdio',
  transportHttp: zhPatched.mcp?.transportHttp || 'HTTP',
  command: zhPatched.mcp?.command || '命令',
  args: zhPatched.mcp?.args || '参数',
  url: zhPatched.mcp?.url || 'URL',
  env: zhPatched.mcp?.env || '环境变量',
  timeout: zhPatched.mcp?.timeout || '超时（秒）',
  enabled: zhPatched.mcp?.enabled || '启用',
  tools: zhPatched.mcp?.tools || '工具数量',
  checkedAt: zhPatched.mcp?.checkedAt || '检查时间',
  stateConnected: zhPatched.mcp?.stateConnected || '已连接',
  stateDegraded: zhPatched.mcp?.stateDegraded || '异常',
  stateDisabled: zhPatched.mcp?.stateDisabled || '已禁用',
  stateInvalid: zhPatched.mcp?.stateInvalid || '无效',
  currentUsing: zhPatched.mcp?.currentUsing || '当前运行集合',
  toggleOn: zhPatched.mcp?.toggleOn || '启用',
  toggleOff: zhPatched.mcp?.toggleOff || '禁用',
  editJson: zhPatched.mcp?.editJson || 'JSON',
  applyJson: zhPatched.mcp?.applyJson || '应用 JSON',
  syncJson: zhPatched.mcp?.syncJson || '从表单同步',
  deleteConfirm: zhPatched.mcp?.deleteConfirm || '确认删除 MCP "{name}"？',
  delete: zhPatched.mcp?.delete || '删除',
  edit: zhPatched.mcp?.edit || '编辑',
  test: zhPatched.mcp?.test || '刷新状态',
  previewOnly:
    zhPatched.mcp?.previewOnly || '浏览器预览模式下不执行真实 MCP 操作，需要在 Tauri 运行时中使用。',
  importHint:
    zhPatched.mcp?.importHint ||
    '支持导入单个 MCP 对象、MCP 字典，或完整配置中的 tools.mcpServers。',
  invalidJson: zhPatched.mcp?.invalidJson || 'JSON 格式无效。',
  importSuccess: zhPatched.mcp?.importSuccess || 'JSON 导入成功。',
  argPlaceholder: zhPatched.mcp?.argPlaceholder || '参数',
  envKeyPlaceholder: zhPatched.mcp?.envKeyPlaceholder || '变量名',
  envValuePlaceholder: zhPatched.mcp?.envValuePlaceholder || '变量值',
  addArg: zhPatched.mcp?.addArg || '添加参数',
  addEnv: zhPatched.mcp?.addEnv || '添加 env',
  noActive: zhPatched.mcp?.noActive || '当前没有启用的 MCP 服务。',
  statusError: zhPatched.mcp?.statusError || '错误',
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
  enable: zhPatched.cron?.enable || '启用',
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
  confirmDelete: zhPatched.cron?.confirmDelete || '确认删除任务 “{name}”？',
  confirmDeleteRunning:
    zhPatched.cron?.confirmDeleteRunning ||
    '任务 “{name}” 正在运行，将先停止再删除，是否继续？',
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

zhPatched.providers = {
  ...(zhPatched.providers || {}),
  currentProvider: zhPatched.providers?.currentProvider || '当前供应商',
  currentModel: zhPatched.providers?.currentModel || '当前模型',
  readiness: zhPatched.providers?.readiness || '就绪状态',
  missingFields: zhPatched.providers?.missingFields || '缺失字段',
  healthReady: zhPatched.providers?.healthReady || 'Doctor 已就绪',
  healthAttention: zhPatched.providers?.healthAttention || 'Doctor 警告',
  resolvedWorkspace: zhPatched.providers?.resolvedWorkspace || '解析工作区',
  missingConfig: zhPatched.providers?.missingConfig || '需要配置',
  ready: zhPatched.providers?.ready || '已就绪',
  none: zhPatched.providers?.none || '无',
  unresolved: zhPatched.providers?.unresolved || '未解析',
  currentTag: zhPatched.providers?.currentTag || '当前',
  manualModelTitle: zhPatched.providers?.manualModelTitle || '手动添加模型',
  manualModelHint:
    zhPatched.providers?.manualModelHint || '当供应商支持某个未出现在目录中的模型时，可直接在这里录入并切换。',
  manualModelPlaceholder:
    zhPatched.providers?.manualModelPlaceholder || '输入供应商原生模型 ID...',
  manualModelAdd: zhPatched.providers?.manualModelAdd || '添加并切换',
  checkToAdd: zhPatched.providers?.checkToAdd || '点击模型即可切换并保存到快捷列表',
  removeCurrentModel: zhPatched.providers?.removeCurrentModel || '删除当前模型',
  refreshModels: zhPatched.providers?.refreshModels || '刷新模型列表',
};

zhPatched.providers.testConnection =
  zhPatched.providers?.testConnection || '娴嬭瘯杩炴帴';
zhPatched.providers.testingConnection =
  zhPatched.providers?.testingConnection || '娴嬭瘯涓?..';
zhPatched.providers.testSuccess =
  zhPatched.providers?.testSuccess || '杩炴帴姝ｅ父';
zhPatched.providers.testSuccessWithLatency =
  zhPatched.providers?.testSuccessWithLatency || '姝ｅ父 {ms}ms';
zhPatched.providers.testFailed =
  zhPatched.providers?.testFailed || '杩炴帴澶辫触';

zhPatched.channels = {
  ...(zhPatched.channels || {}),
  activate: zhPatched.channels?.activate || '激活',
  deactivate: zhPatched.channels?.deactivate || '取消激活',
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
