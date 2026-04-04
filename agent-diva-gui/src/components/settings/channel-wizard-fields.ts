// 通道配置向导表单字段定义
// 用于动态渲染不同平台的配置表单

export interface WizardFormField {
  key: string;
  label: string;
  type?: 'text' | 'password' | 'number' | 'select' | 'textarea';
  secret?: boolean;
  placeholder?: string;
  required?: boolean;
  default?: any;
  hint?: string;
  options?: Array<{ label: string; value: string }>; // 用于 select 类型
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
      key: 'gateway_url',
      label: 'Gateway URL',
      type: 'text',
      required: false,
      default: 'wss://gateway.discord.gg/?v=10&encoding=json',
      hint: 'Discord WebSocket 网关地址',
    },
  ],
  whatsapp: [
    {
      key: 'bridge_url',
      label: '桥接 URL',
      type: 'text',
      required: true,
      placeholder: 'http://localhost:3000',
      hint: 'WhatsApp 桥接服务地址',
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
      required: false,
      placeholder: '飞书验证 Token',
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
      required: false,
      placeholder: '可选：机器人代码',
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
      key: 'imap_use_ssl',
      label: '使用 SSL',
      type: 'select',
      required: true,
      default: true,
      options: [
        { label: '是', value: 'true' },
        { label: '否', value: 'false' },
      ],
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
      key: 'smtp_use_ssl',
      label: 'SMTP 使用 SSL',
      type: 'select',
      required: true,
      default: true,
      options: [
        { label: '是', value: 'true' },
        { label: '否', value: 'false' },
      ],
    },
    {
      key: 'from_address',
      label: '发件人地址',
      type: 'text',
      required: true,
      placeholder: 'your@example.com',
    },
  ],
  slack: [
    {
      key: 'bot_token',
      label: 'Bot Token',
      type: 'password',
      secret: true,
      required: true,
      placeholder: 'xoxb-...',
    },
    {
      key: 'app_token',
      label: 'App Token',
      type: 'password',
      secret: true,
      required: true,
      placeholder: 'xapp-...',
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
      default: 8080,
    },
  ],
  irc: [
    {
      key: 'server',
      label: '服务器',
      type: 'text',
      required: true,
      placeholder: 'irc.libera.chat',
    },
    {
      key: 'port',
      label: '端口',
      type: 'number',
      required: true,
      default: 6667,
    },
    {
      key: 'nickname',
      label: '昵称',
      type: 'text',
      required: true,
      default: 'diva-bot',
    },
    {
      key: 'username',
      label: '用户名',
      type: 'text',
      required: false,
      placeholder: '可选',
    },
    {
      key: 'channels_str',
      label: '频道（逗号分隔）',
      type: 'text',
      required: true,
      placeholder: '#channel1, #channel2',
    },
    {
      key: 'use_tls',
      label: '使用 TLS',
      type: 'select',
      required: true,
      default: false,
      options: [
        { label: '是', value: 'true' },
        { label: '否', value: 'false' },
      ],
    },
  ],
  mattermost: [
    {
      key: 'base_url',
      label: 'Base URL',
      type: 'text',
      required: true,
      placeholder: 'https://mattermost.example.com',
    },
    {
      key: 'bot_token',
      label: 'Bot Token',
      type: 'password',
      secret: true,
      required: true,
    },
    {
      key: 'channel_id',
      label: '频道 ID',
      type: 'text',
      required: false,
    },
    {
      key: 'poll_interval_seconds',
      label: '轮询间隔（秒）',
      type: 'number',
      required: true,
      default: 5,
    },
  ],
  nextcloud_talk: [
    {
      key: 'base_url',
      label: 'Base URL',
      type: 'text',
      required: true,
      placeholder: 'https://cloud.example.com',
    },
    {
      key: 'app_token',
      label: 'App Token',
      type: 'password',
      secret: true,
      required: true,
    },
    {
      key: 'room_token',
      label: '房间令牌',
      type: 'text',
      required: true,
    },
    {
      key: 'poll_interval_seconds',
      label: '轮询间隔（秒）',
      type: 'number',
      required: true,
      default: 5,
    },
  ],
};

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
export function validateConfig(platform: string, config: Record<string, any>): {
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
