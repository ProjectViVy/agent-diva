// 通道配置向导表单字段定义
// 用于动态渲染不同平台的配置表单。字段集合对齐
// agent-diva-core ChannelsConfig 中 GUI 可见通道的 schema。

export type WizardFieldGroup = 'basic' | 'advanced';

export interface WizardFormField {
  key: string;
  label: string;
  type?: 'text' | 'password' | 'number' | 'select' | 'textarea' | 'boolean' | 'string-list';
  secret?: boolean;
  placeholder?: string;
  required?: boolean;
  default?: unknown;
  hint?: string;
  group?: WizardFieldGroup;
  options?: Array<{ label: string; value: string }>;
}

/**
 * 各平台的凭证字段配置
 * key: 平台类型
 * value: 字段数组
 */
export const CHANNEL_CREDENTIAL_FIELDS: Record<string, WizardFormField[]> = {
  telegram: [
    {
      key: 'token',
      label: 'Bot Token',
      type: 'password',
      secret: true,
      required: true,
      placeholder: '输入 Telegram 机器人令牌',
    },
    {
      key: 'allow_from',
      label: '允许的用户 ID',
      type: 'string-list',
      placeholder: '每行一个用户 ID，留空表示不限制',
      hint: '每行一个 ID；留空表示不限制。',
      group: 'advanced',
    },
    {
      key: 'proxy',
      label: '代理 URL',
      type: 'text',
      placeholder: 'http://127.0.0.1:7890',
      group: 'advanced',
    },
  ],
  discord: [
    {
      key: 'token',
      label: 'Bot Token',
      type: 'password',
      secret: true,
      required: true,
      placeholder: '输入 Discord 机器人令牌',
    },
    {
      key: 'allow_from',
      label: '允许的用户 ID',
      type: 'string-list',
      placeholder: '每行一个用户 ID，或 *',
      hint: '每行一个 Discord 用户数字 ID；留空表示不限制。使用 * 表示允许所有人。',
      group: 'advanced',
    },
    {
      key: 'guild_id',
      label: '服务器 ID（可选）',
      type: 'text',
      placeholder: '仅处理该服务器内群消息；留空表示不限制',
      group: 'advanced',
    },
    {
      key: 'mention_only',
      label: '仅在服务器内 @ 机器人时响应',
      type: 'boolean',
      default: false,
      group: 'advanced',
    },
    {
      key: 'listen_to_bots',
      label: '接收其他机器人消息',
      type: 'boolean',
      default: false,
      group: 'advanced',
    },
    {
      key: 'group_reply_allowed_sender_ids',
      label: '免 @ 触发的用户 ID',
      type: 'string-list',
      placeholder: '每行一个用户 ID',
      hint: '在开启「仅 @ 响应」时，这些用户仍可在群内直接发消息触发。',
      group: 'advanced',
    },
    {
      key: 'gateway_url',
      label: 'Gateway URL',
      type: 'text',
      default: 'wss://gateway.discord.gg/?v=10&encoding=json',
      hint: 'Discord WebSocket 网关地址',
      group: 'advanced',
    },
    {
      key: 'intents',
      label: 'Gateway Intents',
      type: 'number',
      default: 37377,
      group: 'advanced',
    },
  ],
  feishu: [
    {
      key: 'app_id',
      label: 'App ID',
      type: 'text',
      required: true,
      placeholder: '飞书应用 App ID',
    },
    {
      key: 'app_secret',
      label: 'App Secret',
      type: 'password',
      secret: true,
      required: true,
      placeholder: '飞书应用 App Secret',
    },
    {
      key: 'verification_token',
      label: 'Verification Token',
      type: 'password',
      secret: true,
      placeholder: '飞书验证 Token',
      group: 'advanced',
    },
    {
      key: 'encrypt_key',
      label: 'Encrypt Key',
      type: 'password',
      secret: true,
      placeholder: '飞书事件加密 Key',
      group: 'advanced',
    },
    {
      key: 'allow_from',
      label: '允许的用户 ID',
      type: 'string-list',
      placeholder: '每行一个用户 ID，留空表示不限制',
      group: 'advanced',
    },
    {
      key: 'port',
      label: 'Webhook 端口（可选）',
      type: 'number',
      hint: '仅 webhook 模式需要；WebSocket 长连接可留空。',
      group: 'advanced',
    },
  ],
  dingtalk: [
    {
      key: 'client_id',
      label: 'Client ID',
      type: 'text',
      required: true,
      placeholder: '钉钉应用 Client ID',
    },
    {
      key: 'client_secret',
      label: 'Client Secret',
      type: 'password',
      secret: true,
      required: true,
      placeholder: '钉钉应用 Client Secret',
    },
    {
      key: 'robot_code',
      label: '机器人代码',
      type: 'text',
      placeholder: '可选：机器人代码',
      group: 'advanced',
    },
    {
      key: 'dm_policy',
      label: '私聊策略',
      type: 'select',
      default: 'open',
      options: [
        { label: '开放', value: 'open' },
        { label: '白名单', value: 'allowlist' },
      ],
      group: 'advanced',
    },
    {
      key: 'group_policy',
      label: '群聊策略',
      type: 'select',
      default: 'open',
      options: [
        { label: '开放', value: 'open' },
        { label: '白名单', value: 'allowlist' },
      ],
      group: 'advanced',
    },
    {
      key: 'allow_from',
      label: '允许的用户 / 群 ID',
      type: 'string-list',
      placeholder: '每行一个 ID，白名单策略时生效',
      group: 'advanced',
    },
  ],
  email: [
    {
      key: 'imap_host',
      label: 'IMAP Host',
      type: 'text',
      required: true,
      placeholder: 'imap.example.com',
    },
    {
      key: 'imap_port',
      label: 'IMAP Port',
      type: 'number',
      required: true,
      default: 993,
    },
    {
      key: 'imap_username',
      label: 'IMAP 用户名',
      type: 'text',
      required: true,
    },
    {
      key: 'imap_password',
      label: 'IMAP 密码',
      type: 'password',
      secret: true,
      required: true,
    },
    {
      key: 'smtp_host',
      label: 'SMTP Host',
      type: 'text',
      required: true,
      placeholder: 'smtp.example.com',
    },
    {
      key: 'smtp_port',
      label: 'SMTP Port',
      type: 'number',
      required: true,
      default: 587,
    },
    {
      key: 'smtp_username',
      label: 'SMTP 用户名',
      type: 'text',
      required: true,
    },
    {
      key: 'smtp_password',
      label: 'SMTP 密码',
      type: 'password',
      secret: true,
      required: true,
    },
    {
      key: 'from_address',
      label: '发件人地址',
      type: 'text',
      required: true,
      placeholder: 'your@example.com',
    },
    {
      key: 'imap_mailbox',
      label: 'IMAP 邮箱',
      type: 'text',
      default: 'INBOX',
      group: 'advanced',
    },
    {
      key: 'imap_use_ssl',
      label: 'IMAP 使用 SSL',
      type: 'boolean',
      default: true,
      group: 'advanced',
    },
    {
      key: 'smtp_use_tls',
      label: 'SMTP 使用 STARTTLS',
      type: 'boolean',
      default: true,
      group: 'advanced',
    },
    {
      key: 'smtp_use_ssl',
      label: 'SMTP 使用 SSL',
      type: 'boolean',
      default: false,
      group: 'advanced',
    },
    {
      key: 'consent_granted',
      label: '已获得邮箱访问授权',
      type: 'boolean',
      default: false,
      hint: '确认已获得访问并发送该邮箱的明确授权。',
      group: 'advanced',
    },
    {
      key: 'auto_reply_enabled',
      label: '启用自动回复',
      type: 'boolean',
      default: true,
      group: 'advanced',
    },
    {
      key: 'poll_interval_seconds',
      label: '轮询间隔（秒）',
      type: 'number',
      default: 30,
      group: 'advanced',
    },
    {
      key: 'mark_seen',
      label: '标记为已读',
      type: 'boolean',
      default: true,
      group: 'advanced',
    },
    {
      key: 'max_body_chars',
      label: '正文最大字符数',
      type: 'number',
      default: 12000,
      group: 'advanced',
    },
    {
      key: 'subject_prefix',
      label: '主题前缀',
      type: 'text',
      default: 'Re: ',
      group: 'advanced',
    },
    {
      key: 'allow_from',
      label: '允许的发件人',
      type: 'string-list',
      placeholder: '每行一个邮箱，留空表示不限制',
      group: 'advanced',
    },
  ],
  qq: [
    {
      key: 'app_id',
      label: 'App ID',
      type: 'text',
      required: true,
      placeholder: 'QQ 机器人 App ID',
    },
    {
      key: 'secret',
      label: '机器人 Secret',
      type: 'password',
      secret: true,
      required: true,
      placeholder: 'QQ 机器人 client secret',
    },
    {
      key: 'allow_from',
      label: '允许的用户 ID',
      type: 'string-list',
      placeholder: '每行一个用户 ID，留空表示不限制',
      group: 'advanced',
    },
  ],
  'neuro-link': [
    {
      key: 'host',
      label: '监听地址',
      type: 'text',
      required: true,
      default: '0.0.0.0',
    },
    {
      key: 'port',
      label: '监听端口',
      type: 'number',
      required: true,
      default: 9100,
    },
    {
      key: 'allow_from',
      label: '允许的客户端',
      type: 'string-list',
      placeholder: '每行一个来源，留空表示不限制',
      group: 'advanced',
    },
  ],
};

export function fieldDefaults(platform: string): Record<string, unknown> {
  const defaults: Record<string, unknown> = {};
  for (const field of CHANNEL_CREDENTIAL_FIELDS[platform] || []) {
    if (field.default !== undefined) {
      defaults[field.key] = field.default;
    } else if (field.type === 'string-list') {
      defaults[field.key] = [];
    } else if (field.type === 'boolean') {
      defaults[field.key] = false;
    }
  }
  return defaults;
}

export function fieldsByGroup(platform: string, group: WizardFieldGroup): WizardFormField[] {
  return (CHANNEL_CREDENTIAL_FIELDS[platform] || []).filter(
    (field) => (field.group ?? 'basic') === group,
  );
}

export function splitIdList(text: string): string[] {
  return text
    .split(/[\n,]+/)
    .map((item) => item.trim())
    .filter(Boolean);
}

export function joinIdList(value: unknown): string {
  if (Array.isArray(value)) {
    return value.map((item) => String(item)).join('\n');
  }
  if (typeof value === 'string') {
    return value;
  }
  return '';
}

export function coerceChannelFieldValue(field: WizardFormField, value: unknown): unknown {
  if (field.type === 'boolean') {
    if (typeof value === 'boolean') return value;
    if (value === 'true') return true;
    if (value === 'false') return false;
    return field.default ?? false;
  }
  if (field.type === 'number') {
    if (typeof value === 'number' && Number.isFinite(value)) return value;
    if (typeof value === 'string' && value.trim() !== '') {
      const parsed = Number(value);
      if (Number.isFinite(parsed)) return parsed;
    }
    return field.default ?? null;
  }
  if (field.type === 'string-list') {
    if (Array.isArray(value)) {
      return value.map((item) => String(item)).filter((item) => item.trim() !== '');
    }
    if (typeof value === 'string') return splitIdList(value);
    return [];
  }
  if (value === undefined || value === null) {
    return field.default ?? '';
  }
  return value;
}

export function normalizeChannelConfig(
  platform: string,
  config: Record<string, unknown>,
): Record<string, unknown> {
  const next: Record<string, unknown> = { ...config };
  for (const field of CHANNEL_CREDENTIAL_FIELDS[platform] || []) {
    if (!(field.key in next)) {
      continue;
    }
    next[field.key] = coerceChannelFieldValue(field, next[field.key]);
  }
  return next;
}

/**
 * 获取平台的必填字段列表
 */
export function getRequiredFields(platform: string): string[] {
  const fields = CHANNEL_CREDENTIAL_FIELDS[platform] || [];
  return fields.filter((f) => f.required).map((f) => f.key);
}

/**
 * 验证配置是否完整
 */
export function validateConfig(
  platform: string,
  config: Record<string, unknown>,
): {
  valid: boolean;
  missing: string[];
} {
  const required = getRequiredFields(platform);
  const missing = required.filter((key) => {
    const value = config[key];
    return value === undefined || value === null || value === '';
  });

  return {
    valid: missing.length === 0,
    missing,
  };
}
