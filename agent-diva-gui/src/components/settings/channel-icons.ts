// 通道平台图标映射表
// 用于在卡片视图和向导中显示各平台的专属图标

import { Mail, Globe, Hash } from 'lucide-vue-next';
import type { Component } from 'vue';
import TelegramIcon from '../assets/icons/channels/TelegramIcon.vue';
import DiscordIcon from '../assets/icons/channels/DiscordIcon.vue';
import SlackIcon from '../assets/icons/channels/SlackIcon.vue';
import DingTalkIcon from '../assets/icons/channels/DingTalkIcon.vue';
import QQIcon from '../assets/icons/channels/QQIcon.vue';
import FeishuIcon from '../assets/icons/channels/FeishuIcon.vue';
import WhatsAppIcon from '../assets/icons/channels/WhatsAppIcon.vue';
import MattermostIcon from '../assets/icons/channels/MattermostIcon.vue';
import NextcloudTalkIcon from '../assets/icons/channels/NextcloudTalkIcon.vue';

/**
 * 平台图标映射
 * key: 通道类型名称（与后端配置一致）
 * value: Vue 图标组件
 */
export const PLATFORM_ICONS: Record<string, Component> = {
  telegram: TelegramIcon,
  discord: DiscordIcon,
  whatsapp: WhatsAppIcon,
  feishu: FeishuIcon, // 飞书
  dingtalk: DingTalkIcon, // 钉钉
  email: Mail, // Email 使用 Lucide Mail 图标
  slack: SlackIcon,
  qq: QQIcon,
  'neuro-link': Globe, // Neuro-Link 使用 Lucide Globe 图标
  irc: Hash, // IRC 使用 Lucide Hash 图标
  mattermost: MattermostIcon,
  nextcloud_talk: NextcloudTalkIcon,
};

/**
 * 平台显示名称映射（用于中文界面）
 */
export const PLATFORM_DISPLAY_NAMES: Record<string, string> = {
  telegram: 'Telegram',
  discord: 'Discord',
  whatsapp: 'WhatsApp',
  feishu: '飞书',
  dingtalk: '钉钉',
  email: 'Email',
  slack: 'Slack',
  qq: 'QQ',
  'neuro-link': 'Neuro-Link',
  irc: 'IRC',
  mattermost: 'Mattermost',
  nextcloud_talk: 'Nextcloud Talk',
};

/**
 * 平台描述映射（用于向导提示）
 */
export const PLATFORM_DESCRIPTIONS: Record<string, string> = {
  telegram: '全球流行的即时通讯平台，支持机器人 API',
  discord: '游戏和社区交流平台，支持丰富的机器人生态',
  whatsapp: '全球广泛使用的即时通讯应用',
  feishu: '企业协作办公平台，支持扫码快速配置',
  dingtalk: '阿里巴巴旗下企业通讯平台',
  email: '传统电子邮件系统（IMAP/SMTP）',
  slack: '企业团队协作聊天工具',
  qq: '腾讯 QQ 开放平台机器人',
  'neuro-link': '通用 WebSocket 接入服务',
  irc: '经典的互联网中继聊天协议',
  mattermost: '开源企业协作平台',
  nextcloud_talk: 'Nextcloud 音视频通话服务',
};
