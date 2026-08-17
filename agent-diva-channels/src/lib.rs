//! Chat platform integrations for agent-diva
//!
//! This crate provides integrations for various chat platforms.
//!
//! RETIRED 2026-08-18（用户决策）：slack / whatsapp / matrix / irc /
//! mattermost / nextcloud_talk 六个未验证通道默认不编译，源文件保留作
//! 历史性保留；需要时启用对应 Cargo feature（`channel-slack` 等）恢复。

pub mod base;
pub mod common;
pub mod dingtalk;
pub mod discord;
pub mod email;
pub mod feishu;
#[cfg(feature = "channel-irc")]
pub mod irc;
pub mod manager;
#[cfg(feature = "channel-matrix")]
pub mod matrix;
#[cfg(feature = "channel-mattermost")]
pub mod mattermost;
pub mod neuro_link;
#[cfg(feature = "channel-nextcloud-talk")]
pub mod nextcloud_talk;
pub mod qq;
#[cfg(feature = "channel-slack")]
pub mod slack;
pub mod telegram;
#[cfg(feature = "channel-whatsapp")]
pub mod whatsapp;

pub use base::{BaseChannel, ChannelError, ChannelHandler, ChannelHandlerPtr, Result};
pub use dingtalk::DingTalkHandler;
pub use discord::DiscordHandler;
pub use email::EmailHandler;
pub use feishu::FeishuHandler;
#[cfg(feature = "channel-irc")]
pub use irc::IrcHandler;
pub use manager::ChannelManager;
#[cfg(feature = "channel-matrix")]
pub use matrix::MatrixHandler;
#[cfg(feature = "channel-mattermost")]
pub use mattermost::MattermostHandler;
pub use neuro_link::NeuroLinkHandler;
#[cfg(feature = "channel-nextcloud-talk")]
pub use nextcloud_talk::NextcloudTalkHandler;
pub use qq::QQHandler;
#[cfg(feature = "channel-slack")]
pub use slack::SlackHandler;
pub use telegram::TelegramHandler;
#[cfg(feature = "channel-whatsapp")]
pub use whatsapp::WhatsAppHandler;
