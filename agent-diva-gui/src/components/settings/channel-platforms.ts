// 通道平台信息定义
// 用于配置向导中的平台信息展示和快速指引

import { CHANNEL_CREDENTIAL_FIELDS } from './channel-wizard-fields';
import type { WizardFormField } from './channel-wizard-fields';
export { CHANNEL_CREDENTIAL_FIELDS };
export type { WizardFormField };

export interface ChannelPlatformInfo {
  name: string;
  displayName: string;
  tutorialPath: string;  // 教程文档路径
  difficulty: 1 | 2 | 3; // 配置难度 1-3 星
  requiresPublicIP: boolean;
  accessMethod: string;  // 接入方式
  credentialFields: WizardFormField[];
  quickGuideSteps: string[];  // 快速获取凭证的步骤数组
}

/**
 * 各平台详细信息
 */
export const CHANNEL_PLATFORMS: Record<string, ChannelPlatformInfo> = {
  telegram: {
    name: 'telegram',
    displayName: 'Telegram',
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
