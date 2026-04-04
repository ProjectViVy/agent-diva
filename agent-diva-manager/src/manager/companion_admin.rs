use tokio::sync::oneshot;

use super::Manager;
use crate::state::SkillUploadRequest;

impl Manager {
    pub(super) fn handle_get_skills(
        &self,
        reply: oneshot::Sender<Result<Vec<crate::skill_service::SkillDto>, String>>,
    ) {
        let result = self
            .skill_service()
            .list_skills()
            .map_err(|e| e.to_string());
        let _ = reply.send(result);
    }

    pub(super) fn handle_get_mcps(
        &self,
        reply: oneshot::Sender<Result<Vec<crate::mcp_service::McpServerDto>, String>>,
    ) {
        let result = self.mcp_service().list_mcps().map_err(|e| e.to_string());
        let _ = reply.send(result);
    }

    pub(super) fn handle_create_mcp(
        &self,
        payload: crate::mcp_service::McpServerUpsert,
        reply: oneshot::Sender<Result<crate::mcp_service::McpServerDto, String>>,
    ) {
        let result = self
            .mcp_service()
            .create_mcp(payload)
            .map_err(|e| e.to_string());
        if result.is_ok() {
            self.reload_runtime_mcp();
        }
        let _ = reply.send(result);
    }

    pub(super) fn handle_update_mcp(
        &self,
        name: String,
        payload: crate::mcp_service::McpServerUpsert,
        reply: oneshot::Sender<Result<crate::mcp_service::McpServerDto, String>>,
    ) {
        let result = self
            .mcp_service()
            .update_mcp(&name, payload)
            .map_err(|e| e.to_string());
        if result.is_ok() {
            self.reload_runtime_mcp();
        }
        let _ = reply.send(result);
    }

    pub(super) fn handle_delete_mcp(
        &self,
        name: String,
        reply: oneshot::Sender<Result<(), String>>,
    ) {
        let result = self
            .mcp_service()
            .delete_mcp(&name)
            .map_err(|e| e.to_string());
        if result.is_ok() {
            self.reload_runtime_mcp();
        }
        let _ = reply.send(result);
    }

    pub(super) fn handle_set_mcp_enabled(
        &self,
        name: String,
        enabled: bool,
        reply: oneshot::Sender<Result<crate::mcp_service::McpServerDto, String>>,
    ) {
        let result = self
            .mcp_service()
            .set_enabled(&name, enabled)
            .map_err(|e| e.to_string());
        if result.is_ok() {
            self.reload_runtime_mcp();
        }
        let _ = reply.send(result);
    }

    pub(super) fn handle_refresh_mcp_status(
        &self,
        name: String,
        reply: oneshot::Sender<Result<crate::mcp_service::McpServerDto, String>>,
    ) {
        let result = self.mcp_service().get_mcp(&name).map_err(|e| e.to_string());
        if result.is_ok() {
            self.reload_runtime_mcp();
        }
        let _ = reply.send(result);
    }

    pub(super) fn handle_upload_skill(
        &self,
        request: SkillUploadRequest,
        reply: oneshot::Sender<Result<crate::skill_service::SkillDto, String>>,
    ) {
        let result = self
            .skill_service()
            .upload_skill_zip(&request.file_name, request.bytes)
            .map_err(|e| e.to_string());
        let _ = reply.send(result);
    }

    pub(super) fn handle_delete_skill(
        &self,
        name: String,
        reply: oneshot::Sender<Result<(), String>>,
    ) {
        let result = self
            .skill_service()
            .delete_skill(&name)
            .map_err(|e| e.to_string());
        let _ = reply.send(result);
    }

    pub(super) async fn handle_upload_file(
        &self,
        request: crate::state::FileUploadRequest,
        reply: oneshot::Sender<Result<agent_diva_core::attachment::FileAttachment, String>>,
    ) {
        let file_service = crate::file_service::FileService::new();
        let result = file_service
            .upload_file(&request.file_name, request.bytes, &request.channel, request.message_id.as_deref())
            .await
            .map_err(|e| e.to_string());
        let _ = reply.send(result);
    }

    fn skill_service(&self) -> crate::skill_service::SkillService {
        crate::skill_service::SkillService::new(self.loader.clone())
    }

    fn mcp_service(&self) -> crate::mcp_service::McpService {
        crate::mcp_service::McpService::new(self.loader.clone())
    }
}
