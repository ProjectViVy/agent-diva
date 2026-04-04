// 通道平台信息定义
// 用于配置向导中的平台信息展示和快速指引

import type { WizardFormField } from './channel-wizard-fields';
export type { WizardFormField };

export interface ChannelPlatformInfo {
  name: string;
  displayName: string;
  icon: string;
  tutorialPath: string;  // 教程文档路径
  difficulty: 1 | 2 | 3; // 配置难度 1-3 星
  requiresPublicIP: boolean;
  accessMethod: string;  // 接入方式
  credentialFields: WizardFormField[];
  quickGuideSteps: string[];  // 快速获取凭证的步骤数组
}

/**
 * 各平台的凭证字段配置
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
 * 各平台详细信息
 */
export const CHANNEL_PLATFORMS: Record<string, ChannelPlatformInfo> = {
  telegram: {
    name: 'telegram',
    displayName: 'Telegram',
    icon: 'telegram',
    tutorialPath: '/docs/channels/telegram.md',
    difficulty: 1,
    requiresPublicIP: false,
    accessMethod: 'Long Polling',
    credentialFields: CHANNEL_CREDENTIAL_FIELDS.telegram,
    quickGuideSteps: [
      '在 Telegram 中搜索 @BotFather 并打开对话',
      '发送 /newbot 命令创建新机器人',
      '按提示设置机器人名称和用户名（以 bot 结尾）',
      '复制 BotFather 返回的 Bot Token',
    ],
  },
  discord: {
    name: 'discord',
    displayName: 'Discord',
    icon: 'discord',
    tutorialPath: '/docs/channels/discord.md',
    difficulty: 2,
    requiresPublicIP: false,
    accessMethod: 'WebSocket Gateway',
    credentialFields: CHANNEL_CREDENTIAL_FIELDS.discord,
    quickGuideSteps: [
      '访问 Discord Developer Portal (https://discord.com/developers)',
      '创建新应用并进入 Bot 页面',
      '点击 "Reset Token" 生成机器人 Token',
      '复制并保存 Token（只显示一次）',
    ],
  },
  whatsapp: {
    name: 'whatsapp',
    displayName: 'WhatsApp',
    icon: 'whatsapp',
    tutorialPath: '/docs/channels/whatsapp.md',
    difficulty: 2,
    requiresPublicIP: false,
    accessMethod: '桥接服务',
    credentialFields: CHANNEL_CREDENTIAL_FIELDS.whatsapp,
    quickGuideSteps: [
      '部署 WhatsApp 桥接服务（如 whatsapp-web.js）',
      '启动桥接服务并获取访问地址',
      '使用手机扫描二维码完成配对',
    ],
  },
  feishu: {
    name: 'feishu',
    displayName: '飞书',
    icon: 'feishu',
    tutorialPath: '/docs/channels/feishu.md',
    difficulty: 2,
    requiresPublicIP: false,
    accessMethod: 'WebSocket 长连接',
    credentialFields: CHANNEL_CREDENTIAL_FIELDS.feishu,
    quickGuideSteps: [
      '登录飞书开放平台 (https://open.feishu.cn)',
      '创建企业自建应用',
      '在"凭证与基础信息"获取 App ID 和 App Secret',
      '添加机器人能力并开通权限',
      '选择"使用长连接接收事件"并添加 im.message.receive_v1',
    ],
  },
  dingtalk: {
    name: 'dingtalk',
    displayName: '钉钉',
    icon: 'dingtalk',
    tutorialPath: '/docs/channels/dingtalk.md',
    difficulty: 2,
    requiresPublicIP: false,
    accessMethod: 'Stream 模式 (WebSocket)',
    credentialFields: CHANNEL_CREDENTIAL_FIELDS.dingtalk,
    quickGuideSteps: [
      '登录钉钉开放平台 (https://open-dev.dingtalk.com)',
      '创建企业内部应用',
      '在"应用凭证"获取 AppKey 和 AppSecret',
      '开启机器人功能并选择"Stream 模式"',
      '发布应用并设置可见范围',
    ],
  },
  email: {
    name: 'email',
    displayName: 'Email',
    icon: 'email',
    tutorialPath: '/docs/channels/email.md',
    difficulty: 2,
    requiresPublicIP: false,
    accessMethod: 'IMAP/SMTP',
    credentialFields: CHANNEL_CREDENTIAL_FIELDS.email,
    quickGuideSteps: [
      '获取邮箱的 IMAP 和 SMTP 服务器地址',
      '在邮箱设置中开启 IMAP/SMTP 服务',
      '生成应用专用密码（推荐使用授权码）',
      '配置收发服务器地址和端口',
    ],
  },
  slack: {
    name: 'slack',
    displayName: 'Slack',
    icon: 'slack',
    tutorialPath: '/docs/channels/slack.md',
    difficulty: 2,
    requiresPublicIP: false,
    accessMethod: 'Socket Mode',
    credentialFields: CHANNEL_CREDENTIAL_FIELDS.slack,
    quickGuideSteps: [
      '访问 Slack API (https://api.slack.com)',
      '创建新应用并添加 Bot 用户',
      '安装应用到工作区',
      '获取 Bot Token 和 App Token',
      '开启 Socket Mode',
    ],
  },
  qq: {
    name: 'qq',
    displayName: 'QQ',
    icon: 'qq',
    tutorialPath: '/docs/channels/qq.md',
    difficulty: 2,
    requiresPublicIP: false,
    accessMethod: 'QQ 开放平台 API',
    credentialFields: CHANNEL_CREDENTIAL_FIELDS.qq,
    quickGuideSteps: [
      '访问 QQ 开放平台 (https://q.qq.com)',
      '创建机器人应用',
      '在开发设置获取 AppID 和 AppSecret',
      '配置功能权限和沙箱环境',
    ],
  },
  'neuro-link': {
    name: 'neuro-link',
    displayName: 'Neuro-Link',
    icon: 'neuro-link',
    tutorialPath: '/docs/channels/neuro-link.md',
    difficulty: 1,
    requiresPublicIP: false,
    accessMethod: 'WebSocket 服务',
    credentialFields: CHANNEL_CREDENTIAL_FIELDS['neuro-link'],
    quickGuideSteps: [
      '配置监听地址（默认 0.0.0.0）',
      '配置监听端口（默认 8080）',
      '启动 Neuro-Link 服务',
    ],
  },
  irc: {
    name: 'irc',
    displayName: 'IRC',
    icon: 'irc',
    tutorialPath: '/docs/channels/irc.md',
    difficulty: 2,
    requiresPublicIP: false,
    accessMethod: 'IRC 协议',
    credentialFields: CHANNEL_CREDENTIAL_FIELDS.irc,
    quickGuideSteps: [
      '选择 IRC 服务器（如 irc.libera.chat）',
      '配置服务器端口（默认 6667）',
      '设置机器人昵称和用户名',
      '配置要加入的频道列表',
    ],
  },
  mattermost: {
    name: 'mattermost',
    displayName: 'Mattermost',
    icon: 'mattermost',
    tutorialPath: '/docs/channels/mattermost.md',
    difficulty: 2,
    requiresPublicIP: false,
    accessMethod: 'Mattermost API',
    credentialFields: CHANNEL_CREDENTIAL_FIELDS.mattermost,
    quickGuideSteps: [
      '配置 Mattermost 服务器地址',
      '创建 Bot 用户并获取 Token',
      '配置要监听的频道 ID',
    ],
  },
  nextcloud_talk: {
    name: 'nextcloud_talk',
    displayName: 'Nextcloud Talk',
    icon: 'nextcloud_talk',
    tutorialPath: '/docs/channels/nextcloud-talk.md',
    difficulty: 2,
    requiresPublicIP: false,
    accessMethod: 'Nextcloud Talk API',
    credentialFields: CHANNEL_CREDENTIAL_FIELDS.nextcloud_talk,
    quickGuideSteps: [
      '配置 Nextcloud 服务器地址',
      '创建应用密码并获取 Token',
      '配置房间令牌（Room Token）',
    ],
  },
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
