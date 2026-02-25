//! Chat platform integrations for agent-diva
//!
//! This crate provides integrations for various chat platforms.

pub mod base;
pub mod common;
pub mod dingtalk;
pub mod discord;
pub mod email;
pub mod feishu;
pub mod generic_pipe;
pub mod irc;
pub mod manager;
pub mod mattermost;
pub mod nextcloud_talk;
pub mod qq;
pub mod slack;
pub mod telegram;
pub mod whatsapp;

pub use base::{BaseChannel, ChannelError, ChannelHandler, ChannelHandlerPtr, Result};
pub use dingtalk::DingTalkHandler;
pub use discord::DiscordHandler;
pub use email::EmailHandler;
pub use feishu::FeishuHandler;
pub use generic_pipe::GenericPipeHandler;
pub use irc::IrcHandler;
pub use manager::ChannelManager;
pub use mattermost::MattermostHandler;
pub use nextcloud_talk::NextcloudTalkHandler;
pub use qq::QQHandler;
pub use telegram::TelegramHandler;
pub use whatsapp::WhatsAppHandler;
