// 通道平台图标映射表
// 用于在卡片视图和向导中显示各平台的专属图标
// 注：slack/whatsapp/nextcloud_talk/mattermost/matrix/irc 已于 2026-08-18
// 从 GUI 下架（后端保留），其图标/名称/描述条目一并移除。

import { Mail, Globe } from '@lucide/vue';
import type { Component } from 'vue';
import TelegramIcon from '../../assets/icons/channels/TelegramIcon.vue';
import DiscordIcon from '../../assets/icons/channels/DiscordIcon.vue';
import DingTalkIcon from '../../assets/icons/channels/DingTalkIcon.vue';
import QQIcon from '../../assets/icons/channels/QQIcon.vue';
import FeishuIcon from '../../assets/icons/channels/FeishuIcon.vue';

/**
 * 平台图标映射
 * key: 通道类型名称（与后端配置一致）
 * value: Vue 图标组件
 */
export const PLATFORM_ICONS: Record<string, Component> = {
  telegram: TelegramIcon,
  discord: DiscordIcon,
  feishu: FeishuIcon, // 飞书
  dingtalk: DingTalkIcon, // 钉钉
  email: Mail, // Email 使用 Lucide Mail 图标
  qq: QQIcon,
  'neuro-link': Globe, // Neuro-Link 使用 Lucide Globe 图标
};

/**
 * 平台显示名称映射（用于中文界面）
 */
export const PLATFORM_DISPLAY_NAMES: Record<string, string> = {
  telegram: 'Telegram',
  discord: 'Discord',
  feishu: '飞书',
  dingtalk: '钉钉',
  email: 'Email',
  qq: 'QQ',
  'neuro-link': 'Neuro-Link',
};

/**
 * 平台描述映射（用于向导提示）
 */
export const PLATFORM_DESCRIPTIONS: Record<string, string> = {
  telegram: '全球流行的即时通讯平台，支持机器人 API',
  discord: '游戏和社区交流平台，支持丰富的机器人生态',
  feishu: '企业协作办公平台，支持扫码快速配置',
  dingtalk: '阿里巴巴旗下企业通讯平台',
  email: '传统电子邮件系统（IMAP/SMTP）',
  qq: '腾讯 QQ 开放平台机器人',
  'neuro-link': '通用 WebSocket 接入服务',
};
