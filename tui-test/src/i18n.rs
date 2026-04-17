//! Internationalization (I18N) module for TUI application.
//!
//! Provides bilingual support (English/Chinese) with runtime language switching.

use std::collections::HashMap;

/// Supported languages
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Language {
    #[default]
    English,
    Chinese,
}

impl Language {
    pub fn code(self) -> &'static str {
        match self {
            Language::English => "en",
            Language::Chinese => "zh",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Language::English => "English",
            Language::Chinese => "中文",
        }
    }

    pub fn from_code(code: &str) -> Self {
        match code {
            "zh" | "zh-CN" | "zh_CN" => Language::Chinese,
            _ => Language::English,
        }
    }
}

/// Translation keys for all UI text
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum TranslationKey {
    // Tab labels (19 tabs)
    TabDashboard,
    TabAgents,
    TabChat,
    TabSessions,
    TabWorkflows,
    TabTriggers,
    TabMemory,
    TabChannels,
    TabSkills,
    TabHands,
    TabExtensions,
    TabTemplates,
    TabPeers,
    TabComms,
    TabSecurity,
    TabAudit,
    TabUsage,
    TabSettings,
    TabLogs,

    // Status badges
    BadgeRun,
    BadgeNew,
    BadgeSus,
    BadgeEnd,
    BadgeErr,
    BadgeUnknown,

    // Welcome screen
    WelcomeTitle,
    WelcomeMenuConnect,
    WelcomeMenuInProcess,
    WelcomeMenuWizard,
    WelcomeMenuLanguage,
    WelcomeMenuExit,
    WelcomeHintConnect,
    WelcomeHintInProcess,
    WelcomeHintWizard,
    WelcomeHintLanguage,
    WelcomeHintExit,
    WelcomeHintNavigate,
    WelcomeCtrlCQuit,
    WelcomeStatusDaemon,
    WelcomeStatusProvider,
    WelcomeStatusMock,
    WelcomeLogoCompact,

    // Settings screen
    SettingsTitle,
    SettingsTabProviders,
    SettingsTabModels,
    SettingsTabTools,
    SettingsTabLanguage,
    SettingsHeaderName,
    SettingsHeaderKey,
    SettingsHeaderModels,
    SettingsHeaderApiBase,
    SettingsHeaderStatus,
    SettingsHeaderEnabled,
    SettingsHeaderApproval,
    SettingsHeaderDescription,
    SettingsHeaderContext,
    SettingsHeaderCost,
    SettingsStatusActive,
    SettingsStatusMissingKey,
    SettingsKeySet,
    SettingsKeyMissing,
    SettingsEnabledOn,
    SettingsEnabledOff,
    SettingsApprovalReq,
    SettingsApprovalAuto,
    SettingsModalSetKey,
    SettingsModalTestProvider,
    SettingsLangSelect,
    SettingsLangCurrent,
    SettingsLangEn,
    SettingsLangZh,
    SettingsHintSetKey,
    SettingsHintTest,
    SettingsHintToggle,
    SettingsHintApproval,
    SettingsHintTab,
    SettingsHintBack,
    SettingsHintProvider,
    SettingsHintKey,
    SettingsTesting,
    SettingsTestSuccess,
    SettingsTestFailed,

    // Dashboard screen
    DashboardTitle,
    DashboardCardAgents,
    DashboardCardUptime,
    DashboardCardProvider,
    DashboardActive,
    DashboardNotSet,
    DashboardAuditLoading,
    DashboardAuditEmpty,
    DashboardHeaderTimestamp,
    DashboardHeaderAgent,
    DashboardHeaderAction,
    DashboardHeaderDetail,
    DashboardHintRefresh,
    DashboardHintAgents,
    DashboardHintScroll,

    // Agents screen
    AgentsTitle,
    AgentsDetailTitle,
    AgentsCreateTitle,
    AgentsTemplatesTitle,
    AgentsCustomName,
    AgentsCustomDesc,
    AgentsCustomPrompt,
    AgentsCustomTools,
    AgentsCustomSkills,
    AgentsCustomMcp,
    AgentsSpawning,
    AgentsHeaderState,
    AgentsHeaderName,
    AgentsHeaderModel,
    AgentsHeaderId,
    AgentsLabelId,
    AgentsLabelName,
    AgentsLabelState,
    AgentsLabelProvider,
    AgentsLabelModel,
    AgentsLabelSkills,
    AgentsLabelMcp,
    AgentsCreateNew,
    AgentsMethodTemplate,
    AgentsMethodCustom,
    AgentsHintTemplate,
    AgentsHintCustom,
    AgentsPromptName,
    AgentsPromptDesc,
    AgentsPromptSys,
    AgentsPromptTools,
    AgentsPromptSkills,
    AgentsPromptMcp,
    AgentsHintNavigate,
    AgentsHintSelect,
    AgentsHintBack,
    AgentsHintToggle,
    AgentsHintNext,
    AgentsHintCreate,
    AgentsHintSave,
    AgentsHintCancel,
    AgentsHintDetail,
    AgentsHintSearch,
    AgentsHintList,
    AgentsEditSkills,
    AgentsEditMcp,
    AgentsNoAgent,
    AgentsNoneAvailable,

    // Chat screen
    ChatTitle,
    ChatSwitchModel,
    ChatNoModels,
    ChatEmptyPrompt,
    ChatHelpHint,
    ChatStaged,
    ChatThinking,
    ChatRunning,
    ChatInputLabel,
    ChatResultLabel,
    ChatErrorLabel,
    ChatTokens,
    ChatHintSend,
    ChatHintModels,
    ChatHintScroll,
    ChatHintStage,
    ChatHintStop,
    ChatHintPicker,

    // Sessions screen
    SessionsTitle,
    SessionsHeaderAgent,
    SessionsHeaderSessionId,
    SessionsHeaderMsgs,
    SessionsHeaderCreated,
    SessionsLoading,
    SessionsEmpty,
    SessionsDeleteConfirm,
    SessionsHintNavigate,
    SessionsHintOpen,
    SessionsHintDelete,
    SessionsHintSearch,
    SessionsYes,
    SessionsCancel,

    // Workflows screen
    WorkflowsTitle,
    WorkflowsRunsTitle,
    WorkflowsNewTitle,
    WorkflowsRunTitle,
    WorkflowsResultTitle,
    WorkflowsHeaderName,
    WorkflowsHeaderDesc,
    WorkflowsHeaderSteps,
    WorkflowsHeaderLastRun,
    WorkflowsHeaderStatus,
    WorkflowsStatusActive,
    WorkflowsStatusDraft,
    WorkflowsStatusPaused,
    WorkflowsNever,
    WorkflowsEmpty,
    WorkflowsLoading,
    WorkflowsRunHeaderId,
    WorkflowsRunHeaderWorkflow,
    WorkflowsRunHeaderStatus,
    WorkflowsRunHeaderStarted,
    WorkflowsRunHeaderDuration,
    WorkflowsRunHeaderProgress,
    WorkflowsRunCompleted,
    WorkflowsRunRunning,
    WorkflowsRunFailed,
    WorkflowsRunPending,
    WorkflowsRunNa,
    WorkflowsRunEmpty,
    WorkflowsCreateTitle,
    WorkflowsCreateName,
    WorkflowsCreateDesc,
    WorkflowsCreateStep,
    WorkflowsCreateNoSteps,
    WorkflowsCreateStepsLabel,
    WorkflowsDeleteConfirm,
    WorkflowsDeleteMsg,
    WorkflowsDeleteWarning,
    WorkflowsDeleteYes,
    WorkflowsDeleteNo,
    WorkflowsHintNew,
    WorkflowsHintRun,
    WorkflowsHintDelete,
    WorkflowsHintRuns,
    WorkflowsHintBack,

    // Memory screen
    MemoryTitle,
    MemorySelectAgent,
    MemoryLoadingAgents,
    MemoryNoAgents,
    MemoryHeaderKey,
    MemoryHeaderValue,
    MemoryLoading,
    MemoryEmpty,
    MemoryDeleteConfirm,
    MemoryAddTitle,
    MemoryEditTitle,
    MemoryPromptKey,
    MemoryPromptValue,
    MemoryHintAdd,
    MemoryHintEdit,
    MemoryHintDelete,
    MemoryHintBack,
    MemoryHintNavigate,

    // Channels screen
    ChannelsTitle,
    ChannelsCategoryAll,
    ChannelsCategoryMessaging,
    ChannelsCategorySocial,
    ChannelsCategoryEnterprise,
    ChannelsCategoryDeveloper,
    ChannelsCategoryNotifications,
    ChannelsHeaderChannel,
    ChannelsHeaderCategory,
    ChannelsHeaderStatus,
    ChannelsHeaderEnvVars,
    ChannelsStatusReady,
    ChannelsStatusMissingEnv,
    ChannelsStatusNotConfigured,
    ChannelsLoading,
    ChannelsSetupTitle,
    ChannelsSetupNoSecrets,
    ChannelsSetupPrompt,
    ChannelsSetupAddConfig,
    ChannelsTesting,
    ChannelsTestingCredentials,
    ChannelsTestPassed,
    ChannelsTestFailed,
    ChannelsHintNavigate,
    ChannelsHintCategory,
    ChannelsHintSetup,

    // Triggers screen
    TriggersTitle,
    TriggersNewTitle,
    TriggersEditTitle,
    TriggersHeaderName,
    TriggersHeaderPatternType,
    TriggersHeaderPattern,
    TriggersHeaderAction,
    TriggersHeaderEnabled,
    TriggersHeaderCount,
    TriggersOn,
    TriggersOff,
    TriggersNever,
    TriggersLoading,
    TriggersEmpty,
    TriggersCreateName,
    TriggersCreatePatternType,
    TriggersCreatePattern,
    TriggersCreateActionType,
    TriggersCreateTarget,
    TriggersCreateEnabled,
    TriggersCreateReview,
    TriggersEditLabel,
    TriggersEditPattern,
    TriggersEditTarget,
    TriggersEditEnabled,
    TriggersDeleteConfirm,
    TriggersDeleteMsg,
    TriggersDeleteWarning,
    TriggersHintNew,
    TriggersHintBack,

    // Skills screen
    SkillsTitle,
    SkillsDetailTitle,
    SkillsSearchTitle,
    SkillsTabInstalled,
    SkillsTabClawHub,
    SkillsTabMcp,
    SkillsHeaderName,
    SkillsHeaderVersion,
    SkillsHeaderStatus,
    SkillsHeaderDesc,
    SkillsHeaderAuthor,
    SkillsStatusActive,
    SkillsStatusInactive,
    SkillsLoading,
    SkillsEmptyInstalled,
    SkillsEmptyClawHub,
    SkillsEmptyMcp,
    SkillsSortName,
    SkillsSortAuthor,
    SkillsSortDate,
    SkillsConfigPlaceholder,
    SkillsSearchPrompt,
    SkillsSearchEmpty,
    SkillsSearchHint,
    SkillsInstallTitle,
    SkillsInstallMsg,
    SkillsInstallDesc,
    SkillsUninstallTitle,
    SkillsUninstallMsg,
    SkillsHintNavigate,
    SkillsHintTab,
    SkillsHintSort,

    // Hands screen
    HandsTitle,
    HandsDetailTitle,
    HandsTabMarketplace,
    HandsTabActive,
    HandsHeaderName,
    HandsHeaderProvider,
    HandsHeaderStatus,
    HandsHeaderDesc,
    HandsHeaderTasks,
    HandsStatusActive,
    HandsStatusPaused,
    HandsStatusInactive,
    HandsNa,
    HandsLoading,
    HandsEmptyMarketplace,
    HandsEmptyActive,
    HandsLabelStatus,
    HandsLabelProvider,
    HandsLabelTasks,
    HandsLabelDesc,
    HandsLabelCapabilities,
    HandsActivateTitle,
    HandsActivateMsg,
    HandsActivateDesc,
    HandsPauseTitle,
    HandsPauseMsg,
    HandsPauseDesc,
    HandsDeactivateTitle,
    HandsDeactivateMsg,
    HandsDeactivateDesc,
    HandsHintNavigate,
    HandsHintTab,

    // Extensions screen
    ExtensionsTitle,
    ExtensionsDetailTitle,
    ExtensionsSearchTitle,
    ExtensionsTabBrowse,
    ExtensionsTabInstalled,
    ExtensionsTabHealth,
    ExtensionsHeaderName,
    ExtensionsHeaderVersion,
    ExtensionsHeaderStatus,
    ExtensionsHeaderDesc,
    ExtensionsHeaderAuthor,
    ExtensionsHealthHealthy,
    ExtensionsHealthError,
    ExtensionsHealthWarning,
    ExtensionsHealthUnknown,
    ExtensionsStatusInstalled,
    ExtensionsStatusAvailable,
    ExtensionsLoading,
    ExtensionsEmptyBrowse,
    ExtensionsEmptyInstalled,
    ExtensionsEmptyHealth,
    ExtensionsLabelStatus,
    ExtensionsLabelId,
    ExtensionsLabelHealth,
    ExtensionsInstallTitle,
    ExtensionsInstallMsg,
    ExtensionsInstallDesc,
    ExtensionsRemoveTitle,
    ExtensionsRemoveMsg,
    ExtensionsRemoveWarning,
    ExtensionsReconnectTitle,
    ExtensionsReconnectMsg,
    ExtensionsReconnectDesc,
    ExtensionsHintNavigate,
    ExtensionsHintTab,

    // Templates screen
    TemplatesTitle,
    TemplatesDetailTitle,
    TemplatesSearchTitle,
    TemplatesCategoryAll,
    TemplatesCategoryAgent,
    TemplatesCategoryWorkflow,
    TemplatesCategorySkill,
    TemplatesCategoryIntegration,
    TemplatesCategoryUtility,
    TemplatesHeaderName,
    TemplatesHeaderCategory,
    TemplatesHeaderDesc,
    TemplatesHeaderPopular,
    TemplatesHeaderAuthor,
    TemplatesLoading,
    TemplatesEmpty,
    TemplatesLabelCategory,
    TemplatesLabelAuthor,
    TemplatesLabelPopularity,
    TemplatesLabelDesc,
    TemplatesLabelTags,
    TemplatesSpawnTitle,
    TemplatesSpawnMsg,
    TemplatesSpawnName,
    TemplatesSpawnDesc,
    TemplatesSearchPrompt,
    TemplatesSearchEmpty,
    TemplatesHintNavigate,
    TemplatesHintSpawn,

    // Peers screen
    PeersTitle,
    PeersHeaderNodeId,
    PeersHeaderName,
    PeersHeaderAddress,
    PeersHeaderState,
    PeersHeaderAgents,
    PeersHeaderProtocol,
    PeersStatusConnected,
    PeersStatusDisconnected,
    PeersStatusConnecting,
    PeersLoading,
    PeersEmpty,
    PeersAutoRefresh,
    PeersHintRefresh,
    PeersHintBack,

    // Comms screen
    CommsTitle,
    CommsTopologyTitle,
    CommsTopologyLoading,
    CommsTopologyEmpty,
    CommsEventsTitle,
    CommsEventsEmpty,
    CommsEventMsg,
    CommsEventSpawned,
    CommsEventKilled,
    CommsEventTask,
    CommsEventClaim,
    CommsEventDone,
    CommsSendTitle,
    CommsSendFrom,
    CommsSendTo,
    CommsSendMsg,
    CommsPostTitle,
    CommsPostTitleLabel,
    CommsPostDesc,
    CommsPostAssign,
    CommsHintNavigate,
    CommsHintSend,
    CommsHintPost,
    CommsHintRefresh,

    // Security screen
    SecurityTitle,
    SecurityFeaturesTitle,
    SecurityHeaderFeature,
    SecurityHeaderStatus,
    SecurityHeaderDesc,
    SecurityInputValidation,
    SecurityRateLimiting,
    SecurityToolSandbox,
    SecurityAuditLogging,
    SecurityMessageSigning,
    SecurityEncryptedStorage,
    SecurityStatusActive,
    SecurityStatusInactive,
    SecurityChainTitle,
    SecurityChainLinks,
    SecurityChainOk,
    SecurityChainErr,
    SecurityChainResultTitle,
    SecurityChainVerified,
    SecurityHintVerify,
    SecurityHintRefresh,
    SecurityHintNavigate,

    // Audit screen
    AuditTitle,
    AuditFilterAll,
    AuditFilterTool,
    AuditFilterAgent,
    AuditFilterMessage,
    AuditFilterConfig,
    AuditFilterSystem,
    AuditHeaderTime,
    AuditHeaderAgent,
    AuditHeaderAction,
    AuditHeaderTarget,
    AuditHeaderChain,
    AuditHeaderHash,
    AuditLoading,
    AuditEmpty,
    AuditSearchPrompt,
    AuditSearchHint,
    AuditDetailTitle,
    AuditDetailTimestamp,
    AuditDetailAgent,
    AuditDetailAction,
    AuditDetailTarget,
    AuditDetailHash,
    AuditDetailChain,
    AuditChainValid,
    AuditChainInvalid,
    AuditChainResultTitle,
    AuditChainVerified,
    AuditChainFailed,
    AuditHintFilter,
    AuditHintSearch,
    AuditHintRefresh,
    AuditHintBack,

    // Usage screen
    UsageTitle,
    UsageTabSummary,
    UsageTabByModel,
    UsageTabByAgent,
    UsageCardRequests,
    UsageCardTokens,
    UsageCardCost,
    UsageCardLatency,
    UsageHeaderModel,
    UsageHeaderProvider,
    UsageHeaderRequests,
    UsageHeaderTokensIn,
    UsageHeaderTokensOut,
    UsageHeaderCost,
    UsageHeaderAgent,
    UsageAutoRefresh,
    UsageHintRefresh,
    UsageHintTab,
    UsageHintBack,

    // Logs screen
    LogsTitle,
    LogsLevel,
    LogsLevelAll,
    LogsLevelError,
    LogsLevelWarn,
    LogsLevelInfo,
    LogsLevelDebug,
    LogsLevelTrace,
    LogsAuto,
    LogsManual,
    LogsEntries,
    LogsLoading,
    LogsEmpty,
    LogsSearchPrompt,
    LogsSearchHint,
    LogsHeaderTime,
    LogsHeaderLevel,
    LogsHeaderSource,
    LogsHeaderMsg,
    LogsHintFilter,
    LogsHintAuto,
    LogsHintSearch,
    LogsHintRefresh,
    LogsHintScroll,
    LogsHintBack,
    LogsHintSearchMode,

    // Placeholder screen
    PlaceholderTitle,
    PlaceholderMsg,
    PlaceholderHintNavigate,
    PlaceholderHintBack,
    PlaceholderHintTab,

    // Wizard (full implementation)
    WizardTitle,
    WizardStepWelcome,
    WizardStepProvider,
    WizardStepApiKey,
    WizardStepModel,
    WizardStepChannel,
    WizardStepAgent,
    WizardStepSummary,
    WizardStepComplete,
    WizardWelcomeMsg,
    WizardWelcomeDesc,
    WizardWelcomeSteps,
    WizardPlaceholderNote,
    WizardHintStart,
    WizardProviderPrompt,
    WizardProviderHint,
    WizardApiKeyProvider,
    WizardApiKeyPrompt,
    WizardApiKeyHint,
    WizardModelPrompt,
    WizardModelHint,
    WizardChannelPrompt,
    WizardChannelHint,
    WizardChannelSetupTitle,
    WizardChannelInputHint,
    WizardAgentPrompt,
    WizardAgentHint,
    WizardAgentHintCustom,
    WizardSummaryTitle,
    WizardSummaryNote,
    WizardSummaryHint,
    WizardCompleteTitle,
    WizardCompleteSuccess,
    WizardCompleteMsg,
    WizardCompleteHint,
    WizardContent,
    WizardHint,

    // Tab bar
    TabBarCtrlCQuit,
    TabBarCtrlCHint,
    TabBarTabHint,

    // Common
    CommonLoading,
    CommonEmpty,
    CommonPlaceholder,
    CommonMock,
    CommonBack,
    CommonCancel,
    CommonSave,
    CommonConfirm,
    CommonYes,
    CommonNo,
    CommonSuccess,
    CommonError,
    CommonPlaceholderAction,
}

/// Translator struct holding current language and translations
pub struct Translator {
    language: Language,
    translations: HashMap<TranslationKey, &'static str>,
}

impl Translator {
    /// Create a new translator with the specified language
    pub fn new(language: Language) -> Self {
        let translations = load_translations(language);
        Self { language, translations }
    }

    /// Get translation for a key
    pub fn t(&self, key: TranslationKey) -> &'static str {
        self.translations.get(&key).copied().unwrap_or("")
    }

    /// Get current language
    pub fn language(&self) -> Language {
        self.language
    }

    /// Switch to a new language
    pub fn set_language(&mut self, language: Language) {
        self.language = language;
        self.translations = load_translations(language);
    }

    /// Format translation with arguments (for dynamic text)
    pub fn t_fmt(&self, key: TranslationKey, args: &[&str]) -> String {
        let template = self.t(key);
        // Simple placeholder replacement: {0}, {1}, etc.
        let mut result = template.to_string();
        for (i, arg) in args.iter().enumerate() {
            result = result.replace(&format!("{{{}}}", i), arg);
        }
        result
    }
}

/// Load translations for a language (compile-time embedded)
fn load_translations(lang: Language) -> HashMap<TranslationKey, &'static str> {
    match lang {
        Language::English => load_english(),
        Language::Chinese => load_chinese(),
    }
}

/// English translations
fn load_english() -> HashMap<TranslationKey, &'static str> {
    let mut map = HashMap::new();

    // Tabs
    map.insert(TranslationKey::TabDashboard, "Dashboard");
    map.insert(TranslationKey::TabAgents, "Agents");
    map.insert(TranslationKey::TabChat, "Chat");
    map.insert(TranslationKey::TabSessions, "Sessions");
    map.insert(TranslationKey::TabWorkflows, "Workflows");
    map.insert(TranslationKey::TabTriggers, "Triggers");
    map.insert(TranslationKey::TabMemory, "Memory");
    map.insert(TranslationKey::TabChannels, "Channels");
    map.insert(TranslationKey::TabSkills, "Skills");
    map.insert(TranslationKey::TabHands, "Hands");
    map.insert(TranslationKey::TabExtensions, "Extensions");
    map.insert(TranslationKey::TabTemplates, "Templates");
    map.insert(TranslationKey::TabPeers, "Peers");
    map.insert(TranslationKey::TabComms, "Comms");
    map.insert(TranslationKey::TabSecurity, "Security");
    map.insert(TranslationKey::TabAudit, "Audit");
    map.insert(TranslationKey::TabUsage, "Usage");
    map.insert(TranslationKey::TabSettings, "Settings");
    map.insert(TranslationKey::TabLogs, "Logs");

    // Badges
    map.insert(TranslationKey::BadgeRun, "[RUN]");
    map.insert(TranslationKey::BadgeNew, "[NEW]");
    map.insert(TranslationKey::BadgeSus, "[SUS]");
    map.insert(TranslationKey::BadgeEnd, "[END]");
    map.insert(TranslationKey::BadgeErr, "[ERR]");
    map.insert(TranslationKey::BadgeUnknown, "[---]");

    // Welcome
    map.insert(TranslationKey::WelcomeTitle, "Agent Operating System");
    map.insert(TranslationKey::WelcomeMenuConnect, "Connect to daemon");
    map.insert(TranslationKey::WelcomeMenuInProcess, "Quick in-process chat");
    map.insert(TranslationKey::WelcomeMenuWizard, "Setup wizard");
    map.insert(TranslationKey::WelcomeMenuLanguage, "Language");
    map.insert(TranslationKey::WelcomeMenuExit, "Exit");
    map.insert(TranslationKey::WelcomeHintConnect, "placeholder - mock connection");
    map.insert(TranslationKey::WelcomeHintInProcess, "placeholder - mock in-process");
    map.insert(TranslationKey::WelcomeHintWizard, "placeholder - configure providers");
    map.insert(TranslationKey::WelcomeHintLanguage, "switch display language");
    map.insert(TranslationKey::WelcomeHintExit, "quit AgentDiVA");
    map.insert(TranslationKey::WelcomeHintNavigate, "navigate  enter select  q quit");
    map.insert(TranslationKey::WelcomeCtrlCQuit, "Press Ctrl+C again to exit");
    map.insert(TranslationKey::WelcomeStatusDaemon, "Placeholder: daemon connection");
    map.insert(TranslationKey::WelcomeStatusProvider, "Provider: Placeholder");
    map.insert(TranslationKey::WelcomeStatusMock, "(mock)");
    map.insert(TranslationKey::WelcomeLogoCompact, "O P E N F A N G");

    // Settings
    map.insert(TranslationKey::SettingsTitle, "Settings");
    map.insert(TranslationKey::SettingsTabProviders, "Providers");
    map.insert(TranslationKey::SettingsTabModels, "Models");
    map.insert(TranslationKey::SettingsTabTools, "Tools");
    map.insert(TranslationKey::SettingsTabLanguage, "Language");
    map.insert(TranslationKey::SettingsHeaderName, "Name");
    map.insert(TranslationKey::SettingsHeaderKey, "Key");
    map.insert(TranslationKey::SettingsHeaderModels, "Models");
    map.insert(TranslationKey::SettingsHeaderApiBase, "API Base");
    map.insert(TranslationKey::SettingsHeaderStatus, "Status");
    map.insert(TranslationKey::SettingsHeaderEnabled, "Enabled");
    map.insert(TranslationKey::SettingsHeaderApproval, "Approval");
    map.insert(TranslationKey::SettingsHeaderDescription, "Description");
    map.insert(TranslationKey::SettingsHeaderContext, "Context");
    map.insert(TranslationKey::SettingsHeaderCost, "Cost/1K");
    map.insert(TranslationKey::SettingsStatusActive, "Active");
    map.insert(TranslationKey::SettingsStatusMissingKey, "Missing Key");
    map.insert(TranslationKey::SettingsKeySet, "[SET]");
    map.insert(TranslationKey::SettingsKeyMissing, "[MISS]");
    map.insert(TranslationKey::SettingsEnabledOn, "[ON]");
    map.insert(TranslationKey::SettingsEnabledOff, "[OFF]");
    map.insert(TranslationKey::SettingsApprovalReq, "[REQ]");
    map.insert(TranslationKey::SettingsApprovalAuto, "[AUTO]");
    map.insert(TranslationKey::SettingsModalSetKey, "Set API Key");
    map.insert(TranslationKey::SettingsModalTestProvider, "Test Provider");
    map.insert(TranslationKey::SettingsLangSelect, "Select Language");
    map.insert(TranslationKey::SettingsLangCurrent, "Current");
    map.insert(TranslationKey::SettingsLangEn, "English");
    map.insert(TranslationKey::SettingsLangZh, "Chinese");
    map.insert(TranslationKey::SettingsHintSetKey, "[k] Set Key");
    map.insert(TranslationKey::SettingsHintTest, "[t] Test");
    map.insert(TranslationKey::SettingsHintToggle, "[Enter/t] Toggle");
    map.insert(TranslationKey::SettingsHintApproval, "[a] Approval");
    map.insert(TranslationKey::SettingsHintTab, "[1-4] Tab");
    map.insert(TranslationKey::SettingsHintBack, "[Esc] Back");
    map.insert(TranslationKey::SettingsHintProvider, "Provider:");
    map.insert(TranslationKey::SettingsHintKey, "Key:");
    map.insert(TranslationKey::SettingsTesting, "Testing...");
    map.insert(TranslationKey::SettingsTestSuccess, "Connection successful! (placeholder)");
    map.insert(TranslationKey::SettingsTestFailed, "No API key set. Please set key first.");

    // Dashboard
    map.insert(TranslationKey::DashboardTitle, "Dashboard");
    map.insert(TranslationKey::DashboardCardAgents, "Agents");
    map.insert(TranslationKey::DashboardCardUptime, "Uptime");
    map.insert(TranslationKey::DashboardCardProvider, "Provider");
    map.insert(TranslationKey::DashboardActive, "active");
    map.insert(TranslationKey::DashboardNotSet, "not set");
    map.insert(TranslationKey::DashboardAuditLoading, "Loading audit trail...");
    map.insert(TranslationKey::DashboardAuditEmpty, "No audit entries yet.");
    map.insert(TranslationKey::DashboardHeaderTimestamp, "Timestamp");
    map.insert(TranslationKey::DashboardHeaderAgent, "Agent");
    map.insert(TranslationKey::DashboardHeaderAction, "Action");
    map.insert(TranslationKey::DashboardHeaderDetail, "Detail");
    map.insert(TranslationKey::DashboardHintRefresh, "[r] Refresh");
    map.insert(TranslationKey::DashboardHintAgents, "[a] Go to Agents");
    map.insert(TranslationKey::DashboardHintScroll, "[j/k] Scroll audit");

    // Agents
    map.insert(TranslationKey::AgentsTitle, "Agents");
    map.insert(TranslationKey::AgentsDetailTitle, "Agent Detail");
    map.insert(TranslationKey::AgentsCreateTitle, "Create Agent");
    map.insert(TranslationKey::AgentsTemplatesTitle, "Templates");
    map.insert(TranslationKey::AgentsCustomName, "Custom - Name");
    map.insert(TranslationKey::AgentsCustomDesc, "Custom - Description");
    map.insert(TranslationKey::AgentsCustomPrompt, "Custom - System Prompt");
    map.insert(TranslationKey::AgentsCustomTools, "Custom - Tools");
    map.insert(TranslationKey::AgentsCustomSkills, "Custom - Skills");
    map.insert(TranslationKey::AgentsCustomMcp, "Custom - MCP Servers");
    map.insert(TranslationKey::AgentsSpawning, "Spawning...");
    map.insert(TranslationKey::AgentsHeaderState, "State");
    map.insert(TranslationKey::AgentsHeaderName, "Name");
    map.insert(TranslationKey::AgentsHeaderModel, "Model");
    map.insert(TranslationKey::AgentsHeaderId, "ID");
    map.insert(TranslationKey::AgentsLabelId, "ID:");
    map.insert(TranslationKey::AgentsLabelName, "Name:");
    map.insert(TranslationKey::AgentsLabelState, "State:");
    map.insert(TranslationKey::AgentsLabelProvider, "Provider:");
    map.insert(TranslationKey::AgentsLabelModel, "Model:");
    map.insert(TranslationKey::AgentsLabelSkills, "Skills:");
    map.insert(TranslationKey::AgentsLabelMcp, "MCP:");
    map.insert(TranslationKey::AgentsCreateNew, "+ Create new agent");
    map.insert(TranslationKey::AgentsMethodTemplate, "Choose from templates");
    map.insert(TranslationKey::AgentsMethodCustom, "Build custom agent");
    map.insert(TranslationKey::AgentsHintTemplate, "(pre-built agents)");
    map.insert(TranslationKey::AgentsHintCustom, "(pick name, tools, prompt)");
    map.insert(TranslationKey::AgentsPromptName, "Agent name:");
    map.insert(TranslationKey::AgentsPromptDesc, "Description:");
    map.insert(TranslationKey::AgentsPromptSys, "System prompt:");
    map.insert(TranslationKey::AgentsPromptTools, "Select tools (Space to toggle):");
    map.insert(TranslationKey::AgentsPromptSkills, "Select skills (none checked = all skills):");
    map.insert(TranslationKey::AgentsPromptMcp, "Select MCP servers (none checked = all servers):");
    map.insert(TranslationKey::AgentsHintNavigate, "Navigate");
    map.insert(TranslationKey::AgentsHintSelect, "Select");
    map.insert(TranslationKey::AgentsHintBack, "Back");
    map.insert(TranslationKey::AgentsHintToggle, "Toggle");
    map.insert(TranslationKey::AgentsHintNext, "Next");
    map.insert(TranslationKey::AgentsHintCreate, "Create");
    map.insert(TranslationKey::AgentsHintSave, "Save");
    map.insert(TranslationKey::AgentsHintCancel, "Cancel");
    map.insert(TranslationKey::AgentsHintDetail, "Edit skills  [m] Edit MCP  [c] Chat  [k] Kill  [Esc] Back");
    map.insert(TranslationKey::AgentsHintSearch, "[Type] Filter  [Enter] Accept  [Esc] Cancel search");
    map.insert(TranslationKey::AgentsHintList, "Navigate  [Enter] Detail  [/] Search  [Esc] Back");
    map.insert(TranslationKey::AgentsEditSkills, "Edit Skills");
    map.insert(TranslationKey::AgentsEditMcp, "Edit MCP Servers");
    map.insert(TranslationKey::AgentsNoAgent, "No agent selected.");
    map.insert(TranslationKey::AgentsNoneAvailable, "(none available)");

    // Chat
    map.insert(TranslationKey::ChatTitle, "Chat");
    map.insert(TranslationKey::ChatSwitchModel, "Switch Model");
    map.insert(TranslationKey::ChatNoModels, "No models match");
    map.insert(TranslationKey::ChatEmptyPrompt, "Send a message to start chatting.");
    map.insert(TranslationKey::ChatHelpHint, "Type /help for available commands.");
    map.insert(TranslationKey::ChatStaged, "(staged)");
    map.insert(TranslationKey::ChatThinking, "thinking...");
    map.insert(TranslationKey::ChatRunning, "running...");
    map.insert(TranslationKey::ChatInputLabel, "input:");
    map.insert(TranslationKey::ChatResultLabel, "result:");
    map.insert(TranslationKey::ChatErrorLabel, "error:");
    map.insert(TranslationKey::ChatTokens, "tokens");
    map.insert(TranslationKey::ChatHintSend, "Send");
    map.insert(TranslationKey::ChatHintModels, "Models");
    map.insert(TranslationKey::ChatHintScroll, "Scroll");
    map.insert(TranslationKey::ChatHintStage, "Stage");
    map.insert(TranslationKey::ChatHintStop, "Stop");
    map.insert(TranslationKey::ChatHintPicker, "Navigate  [Enter] Select  [Esc] Close  [type] Filter");

    // Sessions
    map.insert(TranslationKey::SessionsTitle, "Sessions");
    map.insert(TranslationKey::SessionsHeaderAgent, "Agent");
    map.insert(TranslationKey::SessionsHeaderSessionId, "Session ID");
    map.insert(TranslationKey::SessionsHeaderMsgs, "Msgs");
    map.insert(TranslationKey::SessionsHeaderCreated, "Created");
    map.insert(TranslationKey::SessionsLoading, "Loading sessions...");
    map.insert(TranslationKey::SessionsEmpty, "No sessions found.");
    map.insert(TranslationKey::SessionsDeleteConfirm, "Delete this session? [y] Yes  [any] Cancel");
    map.insert(TranslationKey::SessionsHintNavigate, "Navigate");
    map.insert(TranslationKey::SessionsHintOpen, "Open in Chat");
    map.insert(TranslationKey::SessionsHintDelete, "Delete");
    map.insert(TranslationKey::SessionsHintSearch, "Search");
    map.insert(TranslationKey::SessionsYes, "Yes");
    map.insert(TranslationKey::SessionsCancel, "Cancel");

    // Workflows
    map.insert(TranslationKey::WorkflowsTitle, "Workflows");
    map.insert(TranslationKey::WorkflowsRunsTitle, "Workflow Runs");
    map.insert(TranslationKey::WorkflowsNewTitle, "New Workflow");
    map.insert(TranslationKey::WorkflowsRunTitle, "Run Workflow");
    map.insert(TranslationKey::WorkflowsResultTitle, "Run Result");
    map.insert(TranslationKey::WorkflowsHeaderName, "Name");
    map.insert(TranslationKey::WorkflowsHeaderDesc, "Description");
    map.insert(TranslationKey::WorkflowsHeaderSteps, "Steps");
    map.insert(TranslationKey::WorkflowsHeaderLastRun, "Last Run");
    map.insert(TranslationKey::WorkflowsHeaderStatus, "Status");
    map.insert(TranslationKey::WorkflowsStatusActive, "Active");
    map.insert(TranslationKey::WorkflowsStatusDraft, "Draft");
    map.insert(TranslationKey::WorkflowsStatusPaused, "Paused");
    map.insert(TranslationKey::WorkflowsNever, "Never");
    map.insert(TranslationKey::WorkflowsEmpty, "No workflows. Press [n] to create one.");
    map.insert(TranslationKey::WorkflowsLoading, "Loading workflows...");
    map.insert(TranslationKey::WorkflowsRunHeaderId, "Run ID");
    map.insert(TranslationKey::WorkflowsRunHeaderWorkflow, "Workflow");
    map.insert(TranslationKey::WorkflowsRunHeaderStatus, "Status");
    map.insert(TranslationKey::WorkflowsRunHeaderStarted, "Started");
    map.insert(TranslationKey::WorkflowsRunHeaderDuration, "Duration");
    map.insert(TranslationKey::WorkflowsRunHeaderProgress, "Progress");
    map.insert(TranslationKey::WorkflowsRunCompleted, "Completed");
    map.insert(TranslationKey::WorkflowsRunRunning, "Running");
    map.insert(TranslationKey::WorkflowsRunFailed, "Failed");
    map.insert(TranslationKey::WorkflowsRunPending, "Pending");
    map.insert(TranslationKey::WorkflowsRunNa, "N/A");
    map.insert(TranslationKey::WorkflowsRunEmpty, "No workflow runs yet.");
    map.insert(TranslationKey::WorkflowsCreateTitle, "Create New Workflow");
    map.insert(TranslationKey::WorkflowsCreateName, "Name:");
    map.insert(TranslationKey::WorkflowsCreateDesc, "Description:");
    map.insert(TranslationKey::WorkflowsCreateStep, "Step");
    map.insert(TranslationKey::WorkflowsCreateNoSteps, "(no steps added yet)");
    map.insert(TranslationKey::WorkflowsCreateStepsLabel, "Steps:");
    map.insert(TranslationKey::WorkflowsDeleteConfirm, "Confirm Delete");
    map.insert(TranslationKey::WorkflowsDeleteMsg, "Delete workflow '{0}'?");
    map.insert(TranslationKey::WorkflowsDeleteWarning, "This action cannot be undone.");
    map.insert(TranslationKey::WorkflowsDeleteYes, "[y] Yes");
    map.insert(TranslationKey::WorkflowsDeleteNo, "[n/Esc] No");
    map.insert(TranslationKey::WorkflowsHintNew, "new");
    map.insert(TranslationKey::WorkflowsHintRun, "run");
    map.insert(TranslationKey::WorkflowsHintDelete, "delete");
    map.insert(TranslationKey::WorkflowsHintRuns, "runs");
    map.insert(TranslationKey::WorkflowsHintBack, "back");

    // Memory
    map.insert(TranslationKey::MemoryTitle, "Memory");
    map.insert(TranslationKey::MemorySelectAgent, "Select an agent to browse its memory:");
    map.insert(TranslationKey::MemoryLoadingAgents, "Loading agents...");
    map.insert(TranslationKey::MemoryNoAgents, "No agents available.");
    map.insert(TranslationKey::MemoryHeaderKey, "Key");
    map.insert(TranslationKey::MemoryHeaderValue, "Value");
    map.insert(TranslationKey::MemoryLoading, "Loading...");
    map.insert(TranslationKey::MemoryEmpty, "No key-value pairs. Press [a] to add one.");
    map.insert(TranslationKey::MemoryDeleteConfirm, "Delete this key? [y] Yes  [any] Cancel");
    map.insert(TranslationKey::MemoryAddTitle, "Add Key-Value Pair");
    map.insert(TranslationKey::MemoryEditTitle, "Edit Value");
    map.insert(TranslationKey::MemoryPromptKey, "Key:");
    map.insert(TranslationKey::MemoryPromptValue, "Value:");
    map.insert(TranslationKey::MemoryHintAdd, "Add");
    map.insert(TranslationKey::MemoryHintEdit, "Edit");
    map.insert(TranslationKey::MemoryHintDelete, "Delete");
    map.insert(TranslationKey::MemoryHintBack, "Back");
    map.insert(TranslationKey::MemoryHintNavigate, "Navigate");

    // Channels
    map.insert(TranslationKey::ChannelsTitle, "Channels ({0}/{1} ready)");
    map.insert(TranslationKey::ChannelsCategoryAll, "All");
    map.insert(TranslationKey::ChannelsCategoryMessaging, "Messaging");
    map.insert(TranslationKey::ChannelsCategorySocial, "Social");
    map.insert(TranslationKey::ChannelsCategoryEnterprise, "Enterprise");
    map.insert(TranslationKey::ChannelsCategoryDeveloper, "Developer");
    map.insert(TranslationKey::ChannelsCategoryNotifications, "Notifications");
    map.insert(TranslationKey::ChannelsHeaderChannel, "Channel");
    map.insert(TranslationKey::ChannelsHeaderCategory, "Category");
    map.insert(TranslationKey::ChannelsHeaderStatus, "Status");
    map.insert(TranslationKey::ChannelsHeaderEnvVars, "Env Vars");
    map.insert(TranslationKey::ChannelsStatusReady, "[Ready]");
    map.insert(TranslationKey::ChannelsStatusMissingEnv, "[Missing env]");
    map.insert(TranslationKey::ChannelsStatusNotConfigured, "[Not configured]");
    map.insert(TranslationKey::ChannelsLoading, "Loading channels...");
    map.insert(TranslationKey::ChannelsSetupTitle, "Setup: {0}");
    map.insert(TranslationKey::ChannelsSetupNoSecrets, "This channel has no secret env vars - configure via config.toml");
    map.insert(TranslationKey::ChannelsSetupPrompt, "paste value here...");
    map.insert(TranslationKey::ChannelsSetupAddConfig, "Add to config.toml:");
    map.insert(TranslationKey::ChannelsTesting, "Testing {0}...");
    map.insert(TranslationKey::ChannelsTestingCredentials, "Checking credentials...");
    map.insert(TranslationKey::ChannelsTestPassed, "Test passed");
    map.insert(TranslationKey::ChannelsTestFailed, "Test failed");
    map.insert(TranslationKey::ChannelsHintNavigate, "Navigate");
    map.insert(TranslationKey::ChannelsHintCategory, "Category");
    map.insert(TranslationKey::ChannelsHintSetup, "Setup");

    // Triggers
    map.insert(TranslationKey::TriggersTitle, "Triggers");
    map.insert(TranslationKey::TriggersNewTitle, "New Trigger");
    map.insert(TranslationKey::TriggersEditTitle, "Edit Trigger");
    map.insert(TranslationKey::TriggersHeaderName, "Name");
    map.insert(TranslationKey::TriggersHeaderPatternType, "Pattern Type");
    map.insert(TranslationKey::TriggersHeaderPattern, "Pattern");
    map.insert(TranslationKey::TriggersHeaderAction, "Action");
    map.insert(TranslationKey::TriggersHeaderEnabled, "Enabled");
    map.insert(TranslationKey::TriggersHeaderCount, "Count");
    map.insert(TranslationKey::TriggersOn, "ON");
    map.insert(TranslationKey::TriggersOff, "OFF");
    map.insert(TranslationKey::TriggersNever, "Never");
    map.insert(TranslationKey::TriggersLoading, "Loading triggers...");
    map.insert(TranslationKey::TriggersEmpty, "No triggers. Press [n] to create one.");
    map.insert(TranslationKey::TriggersCreateName, "Trigger Name:");
    map.insert(TranslationKey::TriggersCreatePatternType, "Select Pattern Type:");
    map.insert(TranslationKey::TriggersCreatePattern, "Pattern for {0}:");
    map.insert(TranslationKey::TriggersCreateActionType, "Select Action Type:");
    map.insert(TranslationKey::TriggersCreateTarget, "Target for {0}:");
    map.insert(TranslationKey::TriggersCreateEnabled, "Enabled: [y] Yes / [n] No");
    map.insert(TranslationKey::TriggersCreateReview, "Review & Create:");
    map.insert(TranslationKey::TriggersEditLabel, "Edit Trigger: {0}");
    map.insert(TranslationKey::TriggersEditPattern, "Pattern");
    map.insert(TranslationKey::TriggersEditTarget, "Action Target");
    map.insert(TranslationKey::TriggersEditEnabled, "Enabled");
    map.insert(TranslationKey::TriggersDeleteConfirm, "Confirm Delete");
    map.insert(TranslationKey::TriggersDeleteMsg, "Delete trigger '{0}'?");
    map.insert(TranslationKey::TriggersDeleteWarning, "This action cannot be undone.");
    map.insert(TranslationKey::TriggersHintNew, "new");
    map.insert(TranslationKey::TriggersHintBack, "back");

    // Skills
    map.insert(TranslationKey::SkillsTitle, "Skills");
    map.insert(TranslationKey::SkillsDetailTitle, "Skill Detail");
    map.insert(TranslationKey::SkillsSearchTitle, "Search Skills");
    map.insert(TranslationKey::SkillsTabInstalled, "Installed ({0})");
    map.insert(TranslationKey::SkillsTabClawHub, "ClawHub ({0})");
    map.insert(TranslationKey::SkillsTabMcp, "MCP ({0})");
    map.insert(TranslationKey::SkillsHeaderName, "Name");
    map.insert(TranslationKey::SkillsHeaderVersion, "Version");
    map.insert(TranslationKey::SkillsHeaderStatus, "Status");
    map.insert(TranslationKey::SkillsHeaderDesc, "Description");
    map.insert(TranslationKey::SkillsHeaderAuthor, "Author");
    map.insert(TranslationKey::SkillsStatusActive, "Active");
    map.insert(TranslationKey::SkillsStatusInactive, "Inactive");
    map.insert(TranslationKey::SkillsLoading, "Loading skills...");
    map.insert(TranslationKey::SkillsEmptyInstalled, "No installed skills. Browse ClawHub to install.");
    map.insert(TranslationKey::SkillsEmptyClawHub, "No skills found on ClawHub. Check connection.");
    map.insert(TranslationKey::SkillsEmptyMcp, "No MCP servers configured. Add one in Settings.");
    map.insert(TranslationKey::SkillsSortName, "name");
    map.insert(TranslationKey::SkillsSortAuthor, "author");
    map.insert(TranslationKey::SkillsSortDate, "date");
    map.insert(TranslationKey::SkillsConfigPlaceholder, "Configuration (placeholder):");
    map.insert(TranslationKey::SkillsSearchPrompt, "Search:");
    map.insert(TranslationKey::SkillsSearchEmpty, "No results found.");
    map.insert(TranslationKey::SkillsSearchHint, "Type to search ClawHub skills...");
    map.insert(TranslationKey::SkillsInstallTitle, "Install Skill");
    map.insert(TranslationKey::SkillsInstallMsg, "Install '{0}' from ClawHub?");
    map.insert(TranslationKey::SkillsInstallDesc, "This will add the skill to your installed list.");
    map.insert(TranslationKey::SkillsUninstallTitle, "Uninstall Skill");
    map.insert(TranslationKey::SkillsUninstallMsg, "Uninstall '{0}'?");
    map.insert(TranslationKey::SkillsHintNavigate, "Navigate");
    map.insert(TranslationKey::SkillsHintTab, "Tab");
    map.insert(TranslationKey::SkillsHintSort, "Sort");

    // Hands
    map.insert(TranslationKey::HandsTitle, "Hands");
    map.insert(TranslationKey::HandsDetailTitle, "Hand Detail");
    map.insert(TranslationKey::HandsTabMarketplace, "Marketplace ({0})");
    map.insert(TranslationKey::HandsTabActive, "Active ({0})");
    map.insert(TranslationKey::HandsHeaderName, "Name");
    map.insert(TranslationKey::HandsHeaderProvider, "Provider");
    map.insert(TranslationKey::HandsHeaderStatus, "Status");
    map.insert(TranslationKey::HandsHeaderDesc, "Description");
    map.insert(TranslationKey::HandsHeaderTasks, "Tasks");
    map.insert(TranslationKey::HandsStatusActive, "Active");
    map.insert(TranslationKey::HandsStatusPaused, "Paused");
    map.insert(TranslationKey::HandsStatusInactive, "Inactive");
    map.insert(TranslationKey::HandsNa, "N/A");
    map.insert(TranslationKey::HandsLoading, "Loading hands...");
    map.insert(TranslationKey::HandsEmptyMarketplace, "No hands in marketplace. Check connection.");
    map.insert(TranslationKey::HandsEmptyActive, "No active hands. Activate one from Marketplace.");
    map.insert(TranslationKey::HandsLabelStatus, "Status:");
    map.insert(TranslationKey::HandsLabelProvider, "Provider:");
    map.insert(TranslationKey::HandsLabelTasks, "Tasks Completed:");
    map.insert(TranslationKey::HandsLabelDesc, "Description:");
    map.insert(TranslationKey::HandsLabelCapabilities, "Capabilities:");
    map.insert(TranslationKey::HandsActivateTitle, "Activate Hand");
    map.insert(TranslationKey::HandsActivateMsg, "Activate '{0}'?");
    map.insert(TranslationKey::HandsActivateDesc, "This hand will start processing tasks.");
    map.insert(TranslationKey::HandsPauseTitle, "Pause Hand");
    map.insert(TranslationKey::HandsPauseMsg, "Pause '{0}'?");
    map.insert(TranslationKey::HandsPauseDesc, "The hand will stop processing but remain active.");
    map.insert(TranslationKey::HandsDeactivateTitle, "Deactivate Hand");
    map.insert(TranslationKey::HandsDeactivateMsg, "Deactivate '{0}'?");
    map.insert(TranslationKey::HandsDeactivateDesc, "The hand will return to Marketplace.");
    map.insert(TranslationKey::HandsHintNavigate, "Navigate");
    map.insert(TranslationKey::HandsHintTab, "Tab");

    // Extensions
    map.insert(TranslationKey::ExtensionsTitle, "Extensions");
    map.insert(TranslationKey::ExtensionsDetailTitle, "Extension Detail");
    map.insert(TranslationKey::ExtensionsSearchTitle, "Search Extensions");
    map.insert(TranslationKey::ExtensionsTabBrowse, "Browse ({0})");
    map.insert(TranslationKey::ExtensionsTabInstalled, "Installed ({0})");
    map.insert(TranslationKey::ExtensionsTabHealth, "Health ({0}/{1})");
    map.insert(TranslationKey::ExtensionsHeaderName, "Name");
    map.insert(TranslationKey::ExtensionsHeaderVersion, "Version");
    map.insert(TranslationKey::ExtensionsHeaderStatus, "Status");
    map.insert(TranslationKey::ExtensionsHeaderDesc, "Description");
    map.insert(TranslationKey::ExtensionsHeaderAuthor, "Author");
    map.insert(TranslationKey::ExtensionsHealthHealthy, "Healthy");
    map.insert(TranslationKey::ExtensionsHealthError, "Error");
    map.insert(TranslationKey::ExtensionsHealthWarning, "Warning");
    map.insert(TranslationKey::ExtensionsHealthUnknown, "Unknown");
    map.insert(TranslationKey::ExtensionsStatusInstalled, "Installed");
    map.insert(TranslationKey::ExtensionsStatusAvailable, "Available");
    map.insert(TranslationKey::ExtensionsLoading, "Loading extensions...");
    map.insert(TranslationKey::ExtensionsEmptyBrowse, "No extensions available.");
    map.insert(TranslationKey::ExtensionsEmptyInstalled, "No extensions installed. Browse to install.");
    map.insert(TranslationKey::ExtensionsEmptyHealth, "No health data. Check installed extensions.");
    map.insert(TranslationKey::ExtensionsLabelStatus, "Status:");
    map.insert(TranslationKey::ExtensionsLabelId, "ID:");
    map.insert(TranslationKey::ExtensionsLabelHealth, "Health:");
    map.insert(TranslationKey::ExtensionsInstallTitle, "Install Extension");
    map.insert(TranslationKey::ExtensionsInstallMsg, "Install '{0}'?");
    map.insert(TranslationKey::ExtensionsInstallDesc, "This will add the extension to your installed list.");
    map.insert(TranslationKey::ExtensionsRemoveTitle, "Remove Extension");
    map.insert(TranslationKey::ExtensionsRemoveMsg, "Remove '{0}'?");
    map.insert(TranslationKey::ExtensionsRemoveWarning, "This action cannot be undone.");
    map.insert(TranslationKey::ExtensionsReconnectTitle, "Reconnect Extension");
    map.insert(TranslationKey::ExtensionsReconnectMsg, "Reconnect '{0}'?");
    map.insert(TranslationKey::ExtensionsReconnectDesc, "This will attempt to restore the extension connection.");
    map.insert(TranslationKey::ExtensionsHintNavigate, "Navigate");
    map.insert(TranslationKey::ExtensionsHintTab, "Tab");

    // Templates
    map.insert(TranslationKey::TemplatesTitle, "Templates");
    map.insert(TranslationKey::TemplatesDetailTitle, "Template Detail");
    map.insert(TranslationKey::TemplatesSearchTitle, "Search Templates");
    map.insert(TranslationKey::TemplatesCategoryAll, "All");
    map.insert(TranslationKey::TemplatesCategoryAgent, "Agent");
    map.insert(TranslationKey::TemplatesCategoryWorkflow, "Workflow");
    map.insert(TranslationKey::TemplatesCategorySkill, "Skill");
    map.insert(TranslationKey::TemplatesCategoryIntegration, "Integration");
    map.insert(TranslationKey::TemplatesCategoryUtility, "Utility");
    map.insert(TranslationKey::TemplatesHeaderName, "Name");
    map.insert(TranslationKey::TemplatesHeaderCategory, "Category");
    map.insert(TranslationKey::TemplatesHeaderDesc, "Description");
    map.insert(TranslationKey::TemplatesHeaderPopular, "Popular");
    map.insert(TranslationKey::TemplatesHeaderAuthor, "Author");
    map.insert(TranslationKey::TemplatesLoading, "Loading templates...");
    map.insert(TranslationKey::TemplatesEmpty, "No templates in this category.");
    map.insert(TranslationKey::TemplatesLabelCategory, "Category:");
    map.insert(TranslationKey::TemplatesLabelAuthor, "Author:");
    map.insert(TranslationKey::TemplatesLabelPopularity, "Popularity:");
    map.insert(TranslationKey::TemplatesLabelDesc, "Description:");
    map.insert(TranslationKey::TemplatesLabelTags, "Tags:");
    map.insert(TranslationKey::TemplatesSpawnTitle, "Spawn Template");
    map.insert(TranslationKey::TemplatesSpawnMsg, "Spawn from '{0}'");
    map.insert(TranslationKey::TemplatesSpawnName, "Instance Name:");
    map.insert(TranslationKey::TemplatesSpawnDesc, "This will create a new instance from the template.");
    map.insert(TranslationKey::TemplatesSearchPrompt, "Search:");
    map.insert(TranslationKey::TemplatesSearchEmpty, "No templates found.");
    map.insert(TranslationKey::TemplatesHintNavigate, "Navigate");
    map.insert(TranslationKey::TemplatesHintSpawn, "Spawn");

    // Peers
    map.insert(TranslationKey::PeersTitle, "Peers");
    map.insert(TranslationKey::PeersHeaderNodeId, "Node ID");
    map.insert(TranslationKey::PeersHeaderName, "Name");
    map.insert(TranslationKey::PeersHeaderAddress, "Address");
    map.insert(TranslationKey::PeersHeaderState, "State");
    map.insert(TranslationKey::PeersHeaderAgents, "Agents");
    map.insert(TranslationKey::PeersHeaderProtocol, "Protocol");
    map.insert(TranslationKey::PeersStatusConnected, "Connected");
    map.insert(TranslationKey::PeersStatusDisconnected, "Disconnected");
    map.insert(TranslationKey::PeersStatusConnecting, "Connecting");
    map.insert(TranslationKey::PeersLoading, "Discovering peers...");
    map.insert(TranslationKey::PeersEmpty, "No peers connected. Configure [network] in config.toml to enable OFP.");
    map.insert(TranslationKey::PeersAutoRefresh, "(auto-refreshes every 15s)");
    map.insert(TranslationKey::PeersHintRefresh, "Refresh");
    map.insert(TranslationKey::PeersHintBack, "Back");

    // Comms
    map.insert(TranslationKey::CommsTitle, "Comms");
    map.insert(TranslationKey::CommsTopologyTitle, "Agent Topology ({0} agents, {1} edges)");
    map.insert(TranslationKey::CommsTopologyLoading, "Loading topology...");
    map.insert(TranslationKey::CommsTopologyEmpty, "No agents running.");
    map.insert(TranslationKey::CommsEventsTitle, "Live Event Feed ({0} events)");
    map.insert(TranslationKey::CommsEventsEmpty, "No inter-agent events yet.");
    map.insert(TranslationKey::CommsEventMsg, "MSG");
    map.insert(TranslationKey::CommsEventSpawned, "SPAWNED");
    map.insert(TranslationKey::CommsEventKilled, "KILLED");
    map.insert(TranslationKey::CommsEventTask, "TASK+");
    map.insert(TranslationKey::CommsEventClaim, "CLAIM");
    map.insert(TranslationKey::CommsEventDone, "DONE");
    map.insert(TranslationKey::CommsSendTitle, "Send Message");
    map.insert(TranslationKey::CommsSendFrom, "From (agent ID):");
    map.insert(TranslationKey::CommsSendTo, "To (agent ID):");
    map.insert(TranslationKey::CommsSendMsg, "Message:");
    map.insert(TranslationKey::CommsPostTitle, "Post Task");
    map.insert(TranslationKey::CommsPostTitleLabel, "Title:");
    map.insert(TranslationKey::CommsPostDesc, "Description:");
    map.insert(TranslationKey::CommsPostAssign, "Assign to (agent ID, optional):");
    map.insert(TranslationKey::CommsHintNavigate, "Navigate");
    map.insert(TranslationKey::CommsHintSend, "Send");
    map.insert(TranslationKey::CommsHintPost, "Post");
    map.insert(TranslationKey::CommsHintRefresh, "Refresh");

    // Security
    map.insert(TranslationKey::SecurityTitle, "Security");
    map.insert(TranslationKey::SecurityFeaturesTitle, "Security Features");
    map.insert(TranslationKey::SecurityHeaderFeature, "Feature");
    map.insert(TranslationKey::SecurityHeaderStatus, "Status");
    map.insert(TranslationKey::SecurityHeaderDesc, "Description");
    map.insert(TranslationKey::SecurityInputValidation, "Input Validation");
    map.insert(TranslationKey::SecurityRateLimiting, "Rate Limiting");
    map.insert(TranslationKey::SecurityToolSandbox, "Tool Sandbox");
    map.insert(TranslationKey::SecurityAuditLogging, "Audit Logging");
    map.insert(TranslationKey::SecurityMessageSigning, "Message Signing");
    map.insert(TranslationKey::SecurityEncryptedStorage, "Encrypted Storage");
    map.insert(TranslationKey::SecurityStatusActive, "Active");
    map.insert(TranslationKey::SecurityStatusInactive, "Inactive");
    map.insert(TranslationKey::SecurityChainTitle, "Chain Verification ({0} links)");
    map.insert(TranslationKey::SecurityChainLinks, "({0} links)");
    map.insert(TranslationKey::SecurityChainOk, "[OK]");
    map.insert(TranslationKey::SecurityChainErr, "[ERR]");
    map.insert(TranslationKey::SecurityChainResultTitle, "Chain Verification Result");
    map.insert(TranslationKey::SecurityChainVerified, "All chain links verified. Hash chain integrity confirmed.");
    map.insert(TranslationKey::SecurityHintVerify, "Verify chain");
    map.insert(TranslationKey::SecurityHintRefresh, "Refresh");
    map.insert(TranslationKey::SecurityHintNavigate, "Navigate");

    // Audit
    map.insert(TranslationKey::AuditTitle, "Audit");
    map.insert(TranslationKey::AuditFilterAll, "All");
    map.insert(TranslationKey::AuditFilterTool, "Tool");
    map.insert(TranslationKey::AuditFilterAgent, "Agent");
    map.insert(TranslationKey::AuditFilterMessage, "Message");
    map.insert(TranslationKey::AuditFilterConfig, "Config");
    map.insert(TranslationKey::AuditFilterSystem, "System");
    map.insert(TranslationKey::AuditHeaderTime, "Time");
    map.insert(TranslationKey::AuditHeaderAgent, "Agent");
    map.insert(TranslationKey::AuditHeaderAction, "Action");
    map.insert(TranslationKey::AuditHeaderTarget, "Target");
    map.insert(TranslationKey::AuditHeaderChain, "Chain");
    map.insert(TranslationKey::AuditHeaderHash, "Hash");
    map.insert(TranslationKey::AuditLoading, "Loading audit log...");
    map.insert(TranslationKey::AuditEmpty, "No audit entries for this filter.");
    map.insert(TranslationKey::AuditSearchPrompt, "Search:");
    map.insert(TranslationKey::AuditSearchHint, "Type to search audit entries by agent, action, or target...");
    map.insert(TranslationKey::AuditDetailTitle, "Audit Entry: {0}");
    map.insert(TranslationKey::AuditDetailTimestamp, "Timestamp:");
    map.insert(TranslationKey::AuditDetailAgent, "Agent:");
    map.insert(TranslationKey::AuditDetailAction, "Action:");
    map.insert(TranslationKey::AuditDetailTarget, "Target:");
    map.insert(TranslationKey::AuditDetailHash, "Hash:");
    map.insert(TranslationKey::AuditDetailChain, "Chain:");
    map.insert(TranslationKey::AuditChainValid, "Chain Valid");
    map.insert(TranslationKey::AuditChainInvalid, "Chain Invalid");
    map.insert(TranslationKey::AuditChainResultTitle, "Chain Verification");
    map.insert(TranslationKey::AuditChainVerified, "Hash verified. Chain integrity confirmed.");
    map.insert(TranslationKey::AuditChainFailed, "Hash verification failed. Chain may be corrupted.");
    map.insert(TranslationKey::AuditHintFilter, "Filter");
    map.insert(TranslationKey::AuditHintSearch, "Search");
    map.insert(TranslationKey::AuditHintRefresh, "Refresh");
    map.insert(TranslationKey::AuditHintBack, "Back");

    // Usage
    map.insert(TranslationKey::UsageTitle, "Usage");
    map.insert(TranslationKey::UsageTabSummary, "Summary");
    map.insert(TranslationKey::UsageTabByModel, "By Model");
    map.insert(TranslationKey::UsageTabByAgent, "By Agent");
    map.insert(TranslationKey::UsageCardRequests, "Requests");
    map.insert(TranslationKey::UsageCardTokens, "Tokens");
    map.insert(TranslationKey::UsageCardCost, "Cost");
    map.insert(TranslationKey::UsageCardLatency, "Latency");
    map.insert(TranslationKey::UsageHeaderModel, "Model");
    map.insert(TranslationKey::UsageHeaderProvider, "Provider");
    map.insert(TranslationKey::UsageHeaderRequests, "Requests");
    map.insert(TranslationKey::UsageHeaderTokensIn, "Tokens In");
    map.insert(TranslationKey::UsageHeaderTokensOut, "Tokens Out");
    map.insert(TranslationKey::UsageHeaderCost, "Cost");
    map.insert(TranslationKey::UsageHeaderAgent, "Agent");
    map.insert(TranslationKey::UsageAutoRefresh, "(auto-refresh every 10s)");
    map.insert(TranslationKey::UsageHintRefresh, "Refresh");
    map.insert(TranslationKey::UsageHintTab, "Tab");
    map.insert(TranslationKey::UsageHintBack, "Back");

    // Logs
    map.insert(TranslationKey::LogsTitle, "Logs");
    map.insert(TranslationKey::LogsLevel, "Level:");
    map.insert(TranslationKey::LogsLevelAll, "All");
    map.insert(TranslationKey::LogsLevelError, "ERROR");
    map.insert(TranslationKey::LogsLevelWarn, "WARN");
    map.insert(TranslationKey::LogsLevelInfo, "INFO");
    map.insert(TranslationKey::LogsLevelDebug, "DEBUG");
    map.insert(TranslationKey::LogsLevelTrace, "TRACE");
    map.insert(TranslationKey::LogsAuto, "[AUTO]");
    map.insert(TranslationKey::LogsManual, "[MANUAL]");
    map.insert(TranslationKey::LogsEntries, "entries");
    map.insert(TranslationKey::LogsLoading, "Loading logs...");
    map.insert(TranslationKey::LogsEmpty, "No logs for this filter level.");
    map.insert(TranslationKey::LogsSearchPrompt, "Search:");
    map.insert(TranslationKey::LogsSearchHint, "Type to search logs by source or message...");
    map.insert(TranslationKey::LogsHeaderTime, "Time");
    map.insert(TranslationKey::LogsHeaderLevel, "Level");
    map.insert(TranslationKey::LogsHeaderSource, "Source");
    map.insert(TranslationKey::LogsHeaderMsg, "Message");
    map.insert(TranslationKey::LogsHintFilter, "Filter");
    map.insert(TranslationKey::LogsHintAuto, "Auto-refresh");
    map.insert(TranslationKey::LogsHintSearch, "Search");
    map.insert(TranslationKey::LogsHintRefresh, "Refresh");
    map.insert(TranslationKey::LogsHintScroll, "Scroll");
    map.insert(TranslationKey::LogsHintBack, "Back");
    map.insert(TranslationKey::LogsHintSearchMode, "search  cancel");

    // Placeholder
    map.insert(TranslationKey::PlaceholderTitle, "Coming Soon");
    map.insert(TranslationKey::PlaceholderMsg, "Feature not implemented");
    map.insert(TranslationKey::PlaceholderHintNavigate, "navigate  Back  Tab Switch tab");
    map.insert(TranslationKey::PlaceholderHintBack, "Back");
    map.insert(TranslationKey::PlaceholderHintTab, "Tab");

    // Wizard
    map.insert(TranslationKey::WizardTitle, "Setup Wizard");
    map.insert(TranslationKey::WizardStepWelcome, "Welcome");
    map.insert(TranslationKey::WizardStepProvider, "Select Provider");
    map.insert(TranslationKey::WizardStepApiKey, "Configure API Key");
    map.insert(TranslationKey::WizardStepModel, "Select Models");
    map.insert(TranslationKey::WizardStepChannel, "Configure Channel");
    map.insert(TranslationKey::WizardStepAgent, "Create Agent");
    map.insert(TranslationKey::WizardStepSummary, "Review Configuration");
    map.insert(TranslationKey::WizardStepComplete, "Complete");
    map.insert(TranslationKey::WizardWelcomeMsg, "Let's set up your AgentDiVA environment.");
    map.insert(TranslationKey::WizardWelcomeDesc, "This wizard will guide you through configuring providers, channels, and your first agent.");
    map.insert(TranslationKey::WizardWelcomeSteps, "Steps: Provider \u{2192} API Key \u{2192} Model \u{2192} Channel \u{2192} Agent");
    map.insert(TranslationKey::WizardPlaceholderNote, "Note: This is a demonstration wizard with placeholder data.");
    map.insert(TranslationKey::WizardHintStart, "[Enter] Start setup  [Esc] Exit");
    map.insert(TranslationKey::WizardProviderPrompt, "Select a LLM provider to configure:");
    map.insert(TranslationKey::WizardProviderHint, "[\u{2191}\u{2193}] Navigate  [Enter] Select  [s] Skip  [Esc] Back");
    map.insert(TranslationKey::WizardApiKeyProvider, "Configuring: ");
    map.insert(TranslationKey::WizardApiKeyPrompt, "Enter your API key:");
    map.insert(TranslationKey::WizardApiKeyHint, "[v] Toggle visibility  [t] Test  [Enter] Next  [s] Skip  [Esc] Back");
    map.insert(TranslationKey::WizardModelPrompt, "Select models to enable:");
    map.insert(TranslationKey::WizardModelHint, "[Space] Toggle  [a] All  [n] None  [Enter] Next  [s] Skip  [Esc] Back");
    map.insert(TranslationKey::WizardChannelPrompt, "Select a message channel to configure:");
    map.insert(TranslationKey::WizardChannelHint, "[\u{2191}\u{2193}] Navigate  [Enter] Setup  [s] Skip  [Esc] Back");
    map.insert(TranslationKey::WizardChannelSetupTitle, "Setup: ");
    map.insert(TranslationKey::WizardChannelInputHint, "[Enter] Save value  [Esc] Cancel");
    map.insert(TranslationKey::WizardAgentPrompt, "Create your first agent:");
    map.insert(TranslationKey::WizardAgentHint, "[\u{2191}\u{2193}] Template  [c] Custom  [Enter] Next  [s] Skip  [Esc] Back");
    map.insert(TranslationKey::WizardAgentHintCustom, "[Type] Name  [Enter] Next  [Esc] Templates");
    map.insert(TranslationKey::WizardSummaryTitle, "Configuration Summary");
    map.insert(TranslationKey::WizardSummaryNote, "Configuration will be saved as placeholder data.");
    map.insert(TranslationKey::WizardSummaryHint, "[Enter] Finish  [b] Back to edit  [Esc] Cancel");
    map.insert(TranslationKey::WizardCompleteTitle, "Setup Complete!");
    map.insert(TranslationKey::WizardCompleteSuccess, "Configuration saved successfully!");
    map.insert(TranslationKey::WizardCompleteMsg, "Your AgentDiVA environment is ready.");
    map.insert(TranslationKey::WizardCompleteHint, "[Enter] Enter main screen  [Esc] Review");
    map.insert(TranslationKey::WizardContent, "This is a placeholder wizard screen.");
    map.insert(TranslationKey::WizardHint, "[Esc] Back to Welcome");

    // Tab bar
    map.insert(TranslationKey::TabBarCtrlCQuit, "Press Ctrl+C again to quit");
    map.insert(TranslationKey::TabBarCtrlCHint, "Ctrl+Cx2 quit");
    map.insert(TranslationKey::TabBarTabHint, "Tab/Ctrl+ switch");

    // Common
    map.insert(TranslationKey::CommonLoading, "Loading...");
    map.insert(TranslationKey::CommonEmpty, "No data available.");
    map.insert(TranslationKey::CommonPlaceholder, "placeholder");
    map.insert(TranslationKey::CommonMock, "(mock)");
    map.insert(TranslationKey::CommonBack, "Back");
    map.insert(TranslationKey::CommonCancel, "Cancel");
    map.insert(TranslationKey::CommonSave, "Save");
    map.insert(TranslationKey::CommonConfirm, "Confirm");
    map.insert(TranslationKey::CommonYes, "Yes");
    map.insert(TranslationKey::CommonNo, "No");
    map.insert(TranslationKey::CommonSuccess, "Success");
    map.insert(TranslationKey::CommonError, "Error");
    map.insert(TranslationKey::CommonPlaceholderAction, "(placeholder)");

    map
}

/// Chinese translations
fn load_chinese() -> HashMap<TranslationKey, &'static str> {
    let mut map = HashMap::new();

    // Tabs
    map.insert(TranslationKey::TabDashboard, "仪表盘");
    map.insert(TranslationKey::TabAgents, "智能体");
    map.insert(TranslationKey::TabChat, "对话");
    map.insert(TranslationKey::TabSessions, "会话");
    map.insert(TranslationKey::TabWorkflows, "工作流");
    map.insert(TranslationKey::TabTriggers, "触发器");
    map.insert(TranslationKey::TabMemory, "记忆");
    map.insert(TranslationKey::TabChannels, "通道");
    map.insert(TranslationKey::TabSkills, "技能");
    map.insert(TranslationKey::TabHands, "接管");
    map.insert(TranslationKey::TabExtensions, "扩展");
    map.insert(TranslationKey::TabTemplates, "模板");
    map.insert(TranslationKey::TabPeers, "节点");
    map.insert(TranslationKey::TabComms, "通讯");
    map.insert(TranslationKey::TabSecurity, "安全");
    map.insert(TranslationKey::TabAudit, "审计");
    map.insert(TranslationKey::TabUsage, "用量");
    map.insert(TranslationKey::TabSettings, "设置");
    map.insert(TranslationKey::TabLogs, "日志");

    // Badges
    map.insert(TranslationKey::BadgeRun, "[运行]");
    map.insert(TranslationKey::BadgeNew, "[新建]");
    map.insert(TranslationKey::BadgeSus, "[暂停]");
    map.insert(TranslationKey::BadgeEnd, "[结束]");
    map.insert(TranslationKey::BadgeErr, "[错误]");
    map.insert(TranslationKey::BadgeUnknown, "[---]");

    // Welcome
    map.insert(TranslationKey::WelcomeTitle, "智能体操作系统");
    map.insert(TranslationKey::WelcomeMenuConnect, "连接守护进程");
    map.insert(TranslationKey::WelcomeMenuInProcess, "快速内置对话");
    map.insert(TranslationKey::WelcomeMenuWizard, "设置向导");
    map.insert(TranslationKey::WelcomeMenuLanguage, "语言");
    map.insert(TranslationKey::WelcomeMenuExit, "退出");
    map.insert(TranslationKey::WelcomeHintConnect, "placeholder - 模拟连接");
    map.insert(TranslationKey::WelcomeHintInProcess, "placeholder - 模拟内置");
    map.insert(TranslationKey::WelcomeHintWizard, "placeholder - 配置服务商");
    map.insert(TranslationKey::WelcomeHintLanguage, "切换显示语言");
    map.insert(TranslationKey::WelcomeHintExit, "退出 AgentDiVA");
    map.insert(TranslationKey::WelcomeHintNavigate, "导航  回车 选择  q 退出");
    map.insert(TranslationKey::WelcomeCtrlCQuit, "再次按 Ctrl+C 退出");
    map.insert(TranslationKey::WelcomeStatusDaemon, "Placeholder: 守护进程连接");
    map.insert(TranslationKey::WelcomeStatusProvider, "服务商: Placeholder");
    map.insert(TranslationKey::WelcomeStatusMock, "(模拟)");
    map.insert(TranslationKey::WelcomeLogoCompact, "O P E N F A N G");

    // Settings
    map.insert(TranslationKey::SettingsTitle, "设置");
    map.insert(TranslationKey::SettingsTabProviders, "服务商");
    map.insert(TranslationKey::SettingsTabModels, "模型");
    map.insert(TranslationKey::SettingsTabTools, "工具");
    map.insert(TranslationKey::SettingsTabLanguage, "语言");
    map.insert(TranslationKey::SettingsHeaderName, "名称");
    map.insert(TranslationKey::SettingsHeaderKey, "密钥");
    map.insert(TranslationKey::SettingsHeaderModels, "模型数");
    map.insert(TranslationKey::SettingsHeaderApiBase, "API地址");
    map.insert(TranslationKey::SettingsHeaderStatus, "状态");
    map.insert(TranslationKey::SettingsHeaderEnabled, "启用");
    map.insert(TranslationKey::SettingsHeaderApproval, "审批");
    map.insert(TranslationKey::SettingsHeaderDescription, "描述");
    map.insert(TranslationKey::SettingsHeaderContext, "上下文");
    map.insert(TranslationKey::SettingsHeaderCost, "费用/1K");
    map.insert(TranslationKey::SettingsStatusActive, "正常");
    map.insert(TranslationKey::SettingsStatusMissingKey, "缺少密钥");
    map.insert(TranslationKey::SettingsKeySet, "[已设]");
    map.insert(TranslationKey::SettingsKeyMissing, "[缺失]");
    map.insert(TranslationKey::SettingsEnabledOn, "[开]");
    map.insert(TranslationKey::SettingsEnabledOff, "[关]");
    map.insert(TranslationKey::SettingsApprovalReq, "[需审]");
    map.insert(TranslationKey::SettingsApprovalAuto, "[自动]");
    map.insert(TranslationKey::SettingsModalSetKey, "设置 API 密钥");
    map.insert(TranslationKey::SettingsModalTestProvider, "测试服务商");
    map.insert(TranslationKey::SettingsLangSelect, "选择语言");
    map.insert(TranslationKey::SettingsLangCurrent, "当前");
    map.insert(TranslationKey::SettingsLangEn, "英文");
    map.insert(TranslationKey::SettingsLangZh, "中文");
    map.insert(TranslationKey::SettingsHintSetKey, "[k] 设密钥");
    map.insert(TranslationKey::SettingsHintTest, "[t] 测试");
    map.insert(TranslationKey::SettingsHintToggle, "[回车/t] 切换");
    map.insert(TranslationKey::SettingsHintApproval, "[a] 审批");
    map.insert(TranslationKey::SettingsHintTab, "[1-4] 切换标签");
    map.insert(TranslationKey::SettingsHintBack, "[Esc] 返回");
    map.insert(TranslationKey::SettingsHintProvider, "服务商:");
    map.insert(TranslationKey::SettingsHintKey, "密钥:");
    map.insert(TranslationKey::SettingsTesting, "测试中...");
    map.insert(TranslationKey::SettingsTestSuccess, "连接成功！(placeholder)");
    map.insert(TranslationKey::SettingsTestFailed, "未设置 API 密钥。请先设置密钥。");

    // Dashboard
    map.insert(TranslationKey::DashboardTitle, "仪表盘");
    map.insert(TranslationKey::DashboardCardAgents, "智能体");
    map.insert(TranslationKey::DashboardCardUptime, "运行时间");
    map.insert(TranslationKey::DashboardCardProvider, "服务商");
    map.insert(TranslationKey::DashboardActive, "活跃");
    map.insert(TranslationKey::DashboardNotSet, "未设置");
    map.insert(TranslationKey::DashboardAuditLoading, "加载审计记录...");
    map.insert(TranslationKey::DashboardAuditEmpty, "暂无审计记录。");
    map.insert(TranslationKey::DashboardHeaderTimestamp, "时间戳");
    map.insert(TranslationKey::DashboardHeaderAgent, "智能体");
    map.insert(TranslationKey::DashboardHeaderAction, "操作");
    map.insert(TranslationKey::DashboardHeaderDetail, "详情");
    map.insert(TranslationKey::DashboardHintRefresh, "[r] 刷新");
    map.insert(TranslationKey::DashboardHintAgents, "[a] 进入智能体");
    map.insert(TranslationKey::DashboardHintScroll, "[j/k] 滚动审计");

    // Agents
    map.insert(TranslationKey::AgentsTitle, "智能体");
    map.insert(TranslationKey::AgentsDetailTitle, "智能体详情");
    map.insert(TranslationKey::AgentsCreateTitle, "创建智能体");
    map.insert(TranslationKey::AgentsTemplatesTitle, "模板");
    map.insert(TranslationKey::AgentsCustomName, "自定义 - 名称");
    map.insert(TranslationKey::AgentsCustomDesc, "自定义 - 描述");
    map.insert(TranslationKey::AgentsCustomPrompt, "自定义 - 系统提示");
    map.insert(TranslationKey::AgentsCustomTools, "自定义 - 工具");
    map.insert(TranslationKey::AgentsCustomSkills, "自定义 - 技能");
    map.insert(TranslationKey::AgentsCustomMcp, "自定义 - MCP服务器");
    map.insert(TranslationKey::AgentsSpawning, "创建中...");
    map.insert(TranslationKey::AgentsHeaderState, "状态");
    map.insert(TranslationKey::AgentsHeaderName, "名称");
    map.insert(TranslationKey::AgentsHeaderModel, "模型");
    map.insert(TranslationKey::AgentsHeaderId, "ID");
    map.insert(TranslationKey::AgentsLabelId, "ID:");
    map.insert(TranslationKey::AgentsLabelName, "名称:");
    map.insert(TranslationKey::AgentsLabelState, "状态:");
    map.insert(TranslationKey::AgentsLabelProvider, "服务商:");
    map.insert(TranslationKey::AgentsLabelModel, "模型:");
    map.insert(TranslationKey::AgentsLabelSkills, "技能:");
    map.insert(TranslationKey::AgentsLabelMcp, "MCP:");
    map.insert(TranslationKey::AgentsCreateNew, "+ 创建新智能体");
    map.insert(TranslationKey::AgentsMethodTemplate, "从模板选择");
    map.insert(TranslationKey::AgentsMethodCustom, "自定义构建");
    map.insert(TranslationKey::AgentsHintTemplate, "(预设智能体)");
    map.insert(TranslationKey::AgentsHintCustom, "(选择名称、工具、提示)");
    map.insert(TranslationKey::AgentsPromptName, "智能体名称:");
    map.insert(TranslationKey::AgentsPromptDesc, "描述:");
    map.insert(TranslationKey::AgentsPromptSys, "系统提示:");
    map.insert(TranslationKey::AgentsPromptTools, "选择工具 (空格切换):");
    map.insert(TranslationKey::AgentsPromptSkills, "选择技能 (不选 = 所有技能):");
    map.insert(TranslationKey::AgentsPromptMcp, "选择MCP服务器 (不选 = 所有服务器):");
    map.insert(TranslationKey::AgentsHintNavigate, "导航");
    map.insert(TranslationKey::AgentsHintSelect, "选择");
    map.insert(TranslationKey::AgentsHintBack, "返回");
    map.insert(TranslationKey::AgentsHintToggle, "切换");
    map.insert(TranslationKey::AgentsHintNext, "下一步");
    map.insert(TranslationKey::AgentsHintCreate, "创建");
    map.insert(TranslationKey::AgentsHintSave, "保存");
    map.insert(TranslationKey::AgentsHintCancel, "取消");
    map.insert(TranslationKey::AgentsHintDetail, "编辑技能 [m] 编辑MCP [c] 对话 [k] 终止 [Esc] 返回");
    map.insert(TranslationKey::AgentsHintSearch, "[输入] 筛选 [回车] 确认 [Esc] 取消搜索");
    map.insert(TranslationKey::AgentsHintList, "导航 [回车] 详情 [/] 搜索 [Esc] 返回");
    map.insert(TranslationKey::AgentsEditSkills, "编辑技能");
    map.insert(TranslationKey::AgentsEditMcp, "编辑MCP服务器");
    map.insert(TranslationKey::AgentsNoAgent, "未选择智能体。");
    map.insert(TranslationKey::AgentsNoneAvailable, "(无可用)");

    // Chat
    map.insert(TranslationKey::ChatTitle, "对话");
    map.insert(TranslationKey::ChatSwitchModel, "切换模型");
    map.insert(TranslationKey::ChatNoModels, "无匹配模型");
    map.insert(TranslationKey::ChatEmptyPrompt, "发送消息开始对话。");
    map.insert(TranslationKey::ChatHelpHint, "输入 /help 查看可用命令。");
    map.insert(TranslationKey::ChatStaged, "(暂存)");
    map.insert(TranslationKey::ChatThinking, "思考中...");
    map.insert(TranslationKey::ChatRunning, "运行中...");
    map.insert(TranslationKey::ChatInputLabel, "输入:");
    map.insert(TranslationKey::ChatResultLabel, "结果:");
    map.insert(TranslationKey::ChatErrorLabel, "错误:");
    map.insert(TranslationKey::ChatTokens, "令牌");
    map.insert(TranslationKey::ChatHintSend, "发送");
    map.insert(TranslationKey::ChatHintModels, "模型");
    map.insert(TranslationKey::ChatHintScroll, "滚动");
    map.insert(TranslationKey::ChatHintStage, "暂存");
    map.insert(TranslationKey::ChatHintStop, "停止");
    map.insert(TranslationKey::ChatHintPicker, "导航 [回车] 选择 [Esc] 关闭 [输入] 筛选");

    // Sessions
    map.insert(TranslationKey::SessionsTitle, "会话");
    map.insert(TranslationKey::SessionsHeaderAgent, "智能体");
    map.insert(TranslationKey::SessionsHeaderSessionId, "会话ID");
    map.insert(TranslationKey::SessionsHeaderMsgs, "消息数");
    map.insert(TranslationKey::SessionsHeaderCreated, "创建时间");
    map.insert(TranslationKey::SessionsLoading, "加载会话...");
    map.insert(TranslationKey::SessionsEmpty, "未找到会话。");
    map.insert(TranslationKey::SessionsDeleteConfirm, "删除此会话? [y] 是 [任意] 取消");
    map.insert(TranslationKey::SessionsHintNavigate, "导航");
    map.insert(TranslationKey::SessionsHintOpen, "打开对话");
    map.insert(TranslationKey::SessionsHintDelete, "删除");
    map.insert(TranslationKey::SessionsHintSearch, "搜索");
    map.insert(TranslationKey::SessionsYes, "是");
    map.insert(TranslationKey::SessionsCancel, "取消");

    // Workflows
    map.insert(TranslationKey::WorkflowsTitle, "工作流");
    map.insert(TranslationKey::WorkflowsRunsTitle, "工作流运行");
    map.insert(TranslationKey::WorkflowsNewTitle, "新建工作流");
    map.insert(TranslationKey::WorkflowsRunTitle, "运行工作流");
    map.insert(TranslationKey::WorkflowsResultTitle, "运行结果");
    map.insert(TranslationKey::WorkflowsHeaderName, "名称");
    map.insert(TranslationKey::WorkflowsHeaderDesc, "描述");
    map.insert(TranslationKey::WorkflowsHeaderSteps, "步骤数");
    map.insert(TranslationKey::WorkflowsHeaderLastRun, "最后运行");
    map.insert(TranslationKey::WorkflowsHeaderStatus, "状态");
    map.insert(TranslationKey::WorkflowsStatusActive, "活跃");
    map.insert(TranslationKey::WorkflowsStatusDraft, "草稿");
    map.insert(TranslationKey::WorkflowsStatusPaused, "暂停");
    map.insert(TranslationKey::WorkflowsNever, "从未");
    map.insert(TranslationKey::WorkflowsEmpty, "无工作流。按 [n] 创建。");
    map.insert(TranslationKey::WorkflowsLoading, "加载工作流...");
    map.insert(TranslationKey::WorkflowsRunHeaderId, "运行ID");
    map.insert(TranslationKey::WorkflowsRunHeaderWorkflow, "工作流");
    map.insert(TranslationKey::WorkflowsRunHeaderStatus, "状态");
    map.insert(TranslationKey::WorkflowsRunHeaderStarted, "开始时间");
    map.insert(TranslationKey::WorkflowsRunHeaderDuration, "持续时间");
    map.insert(TranslationKey::WorkflowsRunHeaderProgress, "进度");
    map.insert(TranslationKey::WorkflowsRunCompleted, "已完成");
    map.insert(TranslationKey::WorkflowsRunRunning, "运行中");
    map.insert(TranslationKey::WorkflowsRunFailed, "失败");
    map.insert(TranslationKey::WorkflowsRunPending, "等待中");
    map.insert(TranslationKey::WorkflowsRunNa, "无");
    map.insert(TranslationKey::WorkflowsRunEmpty, "暂无工作流运行记录。");
    map.insert(TranslationKey::WorkflowsCreateTitle, "创建新工作流");
    map.insert(TranslationKey::WorkflowsCreateName, "名称:");
    map.insert(TranslationKey::WorkflowsCreateDesc, "描述:");
    map.insert(TranslationKey::WorkflowsCreateStep, "步骤");
    map.insert(TranslationKey::WorkflowsCreateNoSteps, "(尚未添加步骤)");
    map.insert(TranslationKey::WorkflowsCreateStepsLabel, "步骤:");
    map.insert(TranslationKey::WorkflowsDeleteConfirm, "确认删除");
    map.insert(TranslationKey::WorkflowsDeleteMsg, "删除工作流 '{0}'?");
    map.insert(TranslationKey::WorkflowsDeleteWarning, "此操作无法撤销。");
    map.insert(TranslationKey::WorkflowsDeleteYes, "[y] 是");
    map.insert(TranslationKey::WorkflowsDeleteNo, "[n/Esc] 否");
    map.insert(TranslationKey::WorkflowsHintNew, "新建");
    map.insert(TranslationKey::WorkflowsHintRun, "运行");
    map.insert(TranslationKey::WorkflowsHintDelete, "删除");
    map.insert(TranslationKey::WorkflowsHintRuns, "运行记录");
    map.insert(TranslationKey::WorkflowsHintBack, "返回");

    // Memory
    map.insert(TranslationKey::MemoryTitle, "记忆");
    map.insert(TranslationKey::MemorySelectAgent, "选择智能体以浏览其记忆:");
    map.insert(TranslationKey::MemoryLoadingAgents, "加载智能体...");
    map.insert(TranslationKey::MemoryNoAgents, "无可用智能体。");
    map.insert(TranslationKey::MemoryHeaderKey, "键");
    map.insert(TranslationKey::MemoryHeaderValue, "值");
    map.insert(TranslationKey::MemoryLoading, "加载中...");
    map.insert(TranslationKey::MemoryEmpty, "无键值对。按 [a] 添加。");
    map.insert(TranslationKey::MemoryDeleteConfirm, "删除此键? [y] 是 [任意] 取消");
    map.insert(TranslationKey::MemoryAddTitle, "添加键值对");
    map.insert(TranslationKey::MemoryEditTitle, "编辑值");
    map.insert(TranslationKey::MemoryPromptKey, "键:");
    map.insert(TranslationKey::MemoryPromptValue, "值:");
    map.insert(TranslationKey::MemoryHintAdd, "添加");
    map.insert(TranslationKey::MemoryHintEdit, "编辑");
    map.insert(TranslationKey::MemoryHintDelete, "删除");
    map.insert(TranslationKey::MemoryHintBack, "返回");
    map.insert(TranslationKey::MemoryHintNavigate, "导航");

    // Channels
    map.insert(TranslationKey::ChannelsTitle, "通道 ({0}/{1} 已就绪)");
    map.insert(TranslationKey::ChannelsCategoryAll, "全部");
    map.insert(TranslationKey::ChannelsCategoryMessaging, "消息");
    map.insert(TranslationKey::ChannelsCategorySocial, "社交");
    map.insert(TranslationKey::ChannelsCategoryEnterprise, "企业");
    map.insert(TranslationKey::ChannelsCategoryDeveloper, "开发者");
    map.insert(TranslationKey::ChannelsCategoryNotifications, "通知");
    map.insert(TranslationKey::ChannelsHeaderChannel, "通道");
    map.insert(TranslationKey::ChannelsHeaderCategory, "类别");
    map.insert(TranslationKey::ChannelsHeaderStatus, "状态");
    map.insert(TranslationKey::ChannelsHeaderEnvVars, "环境变量");
    map.insert(TranslationKey::ChannelsStatusReady, "[就绪]");
    map.insert(TranslationKey::ChannelsStatusMissingEnv, "[缺少环境变量]");
    map.insert(TranslationKey::ChannelsStatusNotConfigured, "[未配置]");
    map.insert(TranslationKey::ChannelsLoading, "加载通道...");
    map.insert(TranslationKey::ChannelsSetupTitle, "设置: {0}");
    map.insert(TranslationKey::ChannelsSetupNoSecrets, "此通道无密钥环境变量 - 通过 config.toml 配置");
    map.insert(TranslationKey::ChannelsSetupPrompt, "在此粘贴值...");
    map.insert(TranslationKey::ChannelsSetupAddConfig, "添加到 config.toml:");
    map.insert(TranslationKey::ChannelsTesting, "测试 {0}...");
    map.insert(TranslationKey::ChannelsTestingCredentials, "验证凭据...");
    map.insert(TranslationKey::ChannelsTestPassed, "测试通过");
    map.insert(TranslationKey::ChannelsTestFailed, "测试失败");
    map.insert(TranslationKey::ChannelsHintNavigate, "导航");
    map.insert(TranslationKey::ChannelsHintCategory, "类别");
    map.insert(TranslationKey::ChannelsHintSetup, "设置");

    // Triggers
    map.insert(TranslationKey::TriggersTitle, "触发器");
    map.insert(TranslationKey::TriggersNewTitle, "新建触发器");
    map.insert(TranslationKey::TriggersEditTitle, "编辑触发器");
    map.insert(TranslationKey::TriggersHeaderName, "名称");
    map.insert(TranslationKey::TriggersHeaderPatternType, "模式类型");
    map.insert(TranslationKey::TriggersHeaderPattern, "模式");
    map.insert(TranslationKey::TriggersHeaderAction, "动作");
    map.insert(TranslationKey::TriggersHeaderEnabled, "启用");
    map.insert(TranslationKey::TriggersHeaderCount, "触发次数");
    map.insert(TranslationKey::TriggersOn, "开");
    map.insert(TranslationKey::TriggersOff, "关");
    map.insert(TranslationKey::TriggersNever, "从未");
    map.insert(TranslationKey::TriggersLoading, "加载触发器...");
    map.insert(TranslationKey::TriggersEmpty, "无触发器。按 [n] 创建。");
    map.insert(TranslationKey::TriggersCreateName, "触发器名称:");
    map.insert(TranslationKey::TriggersCreatePatternType, "选择模式类型:");
    map.insert(TranslationKey::TriggersCreatePattern, "{0} 的模式:");
    map.insert(TranslationKey::TriggersCreateActionType, "选择动作类型:");
    map.insert(TranslationKey::TriggersCreateTarget, "{0} 的目标:");
    map.insert(TranslationKey::TriggersCreateEnabled, "启用: [y] 是 / [n] 否");
    map.insert(TranslationKey::TriggersCreateReview, "检查并创建:");
    map.insert(TranslationKey::TriggersEditLabel, "编辑触发器: {0}");
    map.insert(TranslationKey::TriggersEditPattern, "模式");
    map.insert(TranslationKey::TriggersEditTarget, "动作目标");
    map.insert(TranslationKey::TriggersEditEnabled, "启用");
    map.insert(TranslationKey::TriggersDeleteConfirm, "确认删除");
    map.insert(TranslationKey::TriggersDeleteMsg, "删除触发器 '{0}'?");
    map.insert(TranslationKey::TriggersDeleteWarning, "此操作无法撤销。");
    map.insert(TranslationKey::TriggersHintNew, "新建");
    map.insert(TranslationKey::TriggersHintBack, "返回");

    // Skills
    map.insert(TranslationKey::SkillsTitle, "技能");
    map.insert(TranslationKey::SkillsDetailTitle, "技能详情");
    map.insert(TranslationKey::SkillsSearchTitle, "搜索技能");
    map.insert(TranslationKey::SkillsTabInstalled, "已安装 ({0})");
    map.insert(TranslationKey::SkillsTabClawHub, "ClawHub ({0})");
    map.insert(TranslationKey::SkillsTabMcp, "MCP ({0})");
    map.insert(TranslationKey::SkillsHeaderName, "名称");
    map.insert(TranslationKey::SkillsHeaderVersion, "版本");
    map.insert(TranslationKey::SkillsHeaderStatus, "状态");
    map.insert(TranslationKey::SkillsHeaderDesc, "描述");
    map.insert(TranslationKey::SkillsHeaderAuthor, "作者");
    map.insert(TranslationKey::SkillsStatusActive, "活跃");
    map.insert(TranslationKey::SkillsStatusInactive, "未激活");
    map.insert(TranslationKey::SkillsLoading, "加载技能...");
    map.insert(TranslationKey::SkillsEmptyInstalled, "无已安装技能。浏览 ClawHub 安装。");
    map.insert(TranslationKey::SkillsEmptyClawHub, "ClawHub 未找到技能。检查连接。");
    map.insert(TranslationKey::SkillsEmptyMcp, "未配置 MCP 服务器。在设置中添加。");
    map.insert(TranslationKey::SkillsSortName, "名称");
    map.insert(TranslationKey::SkillsSortAuthor, "作者");
    map.insert(TranslationKey::SkillsSortDate, "日期");
    map.insert(TranslationKey::SkillsConfigPlaceholder, "配置 (placeholder):");
    map.insert(TranslationKey::SkillsSearchPrompt, "搜索:");
    map.insert(TranslationKey::SkillsSearchEmpty, "无搜索结果。");
    map.insert(TranslationKey::SkillsSearchHint, "输入以搜索 ClawHub 技能...");
    map.insert(TranslationKey::SkillsInstallTitle, "安装技能");
    map.insert(TranslationKey::SkillsInstallMsg, "从 ClawHub 安装 '{0}'?");
    map.insert(TranslationKey::SkillsInstallDesc, "这将添加技能到已安装列表。");
    map.insert(TranslationKey::SkillsUninstallTitle, "卸载技能");
    map.insert(TranslationKey::SkillsUninstallMsg, "卸载 '{0}'?");
    map.insert(TranslationKey::SkillsHintNavigate, "导航");
    map.insert(TranslationKey::SkillsHintTab, "标签");
    map.insert(TranslationKey::SkillsHintSort, "排序");

    // Hands
    map.insert(TranslationKey::HandsTitle, "接管");
    map.insert(TranslationKey::HandsDetailTitle, "接管详情");
    map.insert(TranslationKey::HandsTabMarketplace, "市场 ({0})");
    map.insert(TranslationKey::HandsTabActive, "活跃 ({0})");
    map.insert(TranslationKey::HandsHeaderName, "名称");
    map.insert(TranslationKey::HandsHeaderProvider, "服务商");
    map.insert(TranslationKey::HandsHeaderStatus, "状态");
    map.insert(TranslationKey::HandsHeaderDesc, "描述");
    map.insert(TranslationKey::HandsHeaderTasks, "任务数");
    map.insert(TranslationKey::HandsStatusActive, "活跃");
    map.insert(TranslationKey::HandsStatusPaused, "暂停");
    map.insert(TranslationKey::HandsStatusInactive, "未激活");
    map.insert(TranslationKey::HandsNa, "无");
    map.insert(TranslationKey::HandsLoading, "加载接管...");
    map.insert(TranslationKey::HandsEmptyMarketplace, "市场无接管。检查连接。");
    map.insert(TranslationKey::HandsEmptyActive, "无活跃接管。从市场激活。");
    map.insert(TranslationKey::HandsLabelStatus, "状态:");
    map.insert(TranslationKey::HandsLabelProvider, "服务商:");
    map.insert(TranslationKey::HandsLabelTasks, "已完成任务:");
    map.insert(TranslationKey::HandsLabelDesc, "描述:");
    map.insert(TranslationKey::HandsLabelCapabilities, "能力:");
    map.insert(TranslationKey::HandsActivateTitle, "激活接管");
    map.insert(TranslationKey::HandsActivateMsg, "激活 '{0}'?");
    map.insert(TranslationKey::HandsActivateDesc, "此接管将开始处理任务。");
    map.insert(TranslationKey::HandsPauseTitle, "暂停接管");
    map.insert(TranslationKey::HandsPauseMsg, "暂停 '{0}'?");
    map.insert(TranslationKey::HandsPauseDesc, "接管将停止处理但保持活跃状态。");
    map.insert(TranslationKey::HandsDeactivateTitle, "停用接管");
    map.insert(TranslationKey::HandsDeactivateMsg, "停用 '{0}'?");
    map.insert(TranslationKey::HandsDeactivateDesc, "接管将返回市场。");
    map.insert(TranslationKey::HandsHintNavigate, "导航");
    map.insert(TranslationKey::HandsHintTab, "标签");

    // Extensions
    map.insert(TranslationKey::ExtensionsTitle, "扩展");
    map.insert(TranslationKey::ExtensionsDetailTitle, "扩展详情");
    map.insert(TranslationKey::ExtensionsSearchTitle, "搜索扩展");
    map.insert(TranslationKey::ExtensionsTabBrowse, "浏览 ({0})");
    map.insert(TranslationKey::ExtensionsTabInstalled, "已安装 ({0})");
    map.insert(TranslationKey::ExtensionsTabHealth, "健康 ({0}/{1})");
    map.insert(TranslationKey::ExtensionsHeaderName, "名称");
    map.insert(TranslationKey::ExtensionsHeaderVersion, "版本");
    map.insert(TranslationKey::ExtensionsHeaderStatus, "状态");
    map.insert(TranslationKey::ExtensionsHeaderDesc, "描述");
    map.insert(TranslationKey::ExtensionsHeaderAuthor, "作者");
    map.insert(TranslationKey::ExtensionsHealthHealthy, "健康");
    map.insert(TranslationKey::ExtensionsHealthError, "错误");
    map.insert(TranslationKey::ExtensionsHealthWarning, "警告");
    map.insert(TranslationKey::ExtensionsHealthUnknown, "未知");
    map.insert(TranslationKey::ExtensionsStatusInstalled, "已安装");
    map.insert(TranslationKey::ExtensionsStatusAvailable, "可用");
    map.insert(TranslationKey::ExtensionsLoading, "加载扩展...");
    map.insert(TranslationKey::ExtensionsEmptyBrowse, "无可用扩展。");
    map.insert(TranslationKey::ExtensionsEmptyInstalled, "无已安装扩展。浏览安装。");
    map.insert(TranslationKey::ExtensionsEmptyHealth, "无健康数据。检查已安装扩展。");
    map.insert(TranslationKey::ExtensionsLabelStatus, "状态:");
    map.insert(TranslationKey::ExtensionsLabelId, "ID:");
    map.insert(TranslationKey::ExtensionsLabelHealth, "健康:");
    map.insert(TranslationKey::ExtensionsInstallTitle, "安装扩展");
    map.insert(TranslationKey::ExtensionsInstallMsg, "安装 '{0}'?");
    map.insert(TranslationKey::ExtensionsInstallDesc, "这将添加扩展到已安装列表。");
    map.insert(TranslationKey::ExtensionsRemoveTitle, "移除扩展");
    map.insert(TranslationKey::ExtensionsRemoveMsg, "移除 '{0}'?");
    map.insert(TranslationKey::ExtensionsRemoveWarning, "此操作无法撤销。");
    map.insert(TranslationKey::ExtensionsReconnectTitle, "重连扩展");
    map.insert(TranslationKey::ExtensionsReconnectMsg, "重连 '{0}'?");
    map.insert(TranslationKey::ExtensionsReconnectDesc, "尝试恢复扩展连接。");
    map.insert(TranslationKey::ExtensionsHintNavigate, "导航");
    map.insert(TranslationKey::ExtensionsHintTab, "标签");

    // Templates
    map.insert(TranslationKey::TemplatesTitle, "模板");
    map.insert(TranslationKey::TemplatesDetailTitle, "模板详情");
    map.insert(TranslationKey::TemplatesSearchTitle, "搜索模板");
    map.insert(TranslationKey::TemplatesCategoryAll, "全部");
    map.insert(TranslationKey::TemplatesCategoryAgent, "智能体");
    map.insert(TranslationKey::TemplatesCategoryWorkflow, "工作流");
    map.insert(TranslationKey::TemplatesCategorySkill, "技能");
    map.insert(TranslationKey::TemplatesCategoryIntegration, "集成");
    map.insert(TranslationKey::TemplatesCategoryUtility, "工具");
    map.insert(TranslationKey::TemplatesHeaderName, "名称");
    map.insert(TranslationKey::TemplatesHeaderCategory, "类别");
    map.insert(TranslationKey::TemplatesHeaderDesc, "描述");
    map.insert(TranslationKey::TemplatesHeaderPopular, "热度");
    map.insert(TranslationKey::TemplatesHeaderAuthor, "作者");
    map.insert(TranslationKey::TemplatesLoading, "加载模板...");
    map.insert(TranslationKey::TemplatesEmpty, "此类别无模板。");
    map.insert(TranslationKey::TemplatesLabelCategory, "类别:");
    map.insert(TranslationKey::TemplatesLabelAuthor, "作者:");
    map.insert(TranslationKey::TemplatesLabelPopularity, "热度:");
    map.insert(TranslationKey::TemplatesLabelDesc, "描述:");
    map.insert(TranslationKey::TemplatesLabelTags, "标签:");
    map.insert(TranslationKey::TemplatesSpawnTitle, "生成模板");
    map.insert(TranslationKey::TemplatesSpawnMsg, "从 '{0}' 生成");
    map.insert(TranslationKey::TemplatesSpawnName, "实例名称:");
    map.insert(TranslationKey::TemplatesSpawnDesc, "将从模板创建新实例。");
    map.insert(TranslationKey::TemplatesSearchPrompt, "搜索:");
    map.insert(TranslationKey::TemplatesSearchEmpty, "无搜索结果。");
    map.insert(TranslationKey::TemplatesHintNavigate, "导航");
    map.insert(TranslationKey::TemplatesHintSpawn, "生成");

    // Peers
    map.insert(TranslationKey::PeersTitle, "节点");
    map.insert(TranslationKey::PeersHeaderNodeId, "节点ID");
    map.insert(TranslationKey::PeersHeaderName, "名称");
    map.insert(TranslationKey::PeersHeaderAddress, "地址");
    map.insert(TranslationKey::PeersHeaderState, "状态");
    map.insert(TranslationKey::PeersHeaderAgents, "智能体");
    map.insert(TranslationKey::PeersHeaderProtocol, "协议");
    map.insert(TranslationKey::PeersStatusConnected, "已连接");
    map.insert(TranslationKey::PeersStatusDisconnected, "已断开");
    map.insert(TranslationKey::PeersStatusConnecting, "连接中");
    map.insert(TranslationKey::PeersLoading, "发现节点...");
    map.insert(TranslationKey::PeersEmpty, "无已连接节点。配置 config.toml [network] 启用 OFP。");
    map.insert(TranslationKey::PeersAutoRefresh, "(每15秒自动刷新)");
    map.insert(TranslationKey::PeersHintRefresh, "刷新");
    map.insert(TranslationKey::PeersHintBack, "返回");

    // Comms
    map.insert(TranslationKey::CommsTitle, "通讯");
    map.insert(TranslationKey::CommsTopologyTitle, "智能体拓扑 ({0} 智能体, {1} 边)");
    map.insert(TranslationKey::CommsTopologyLoading, "加载拓扑...");
    map.insert(TranslationKey::CommsTopologyEmpty, "无运行智能体。");
    map.insert(TranslationKey::CommsEventsTitle, "实时事件流 ({0} 事件)");
    map.insert(TranslationKey::CommsEventsEmpty, "暂无智能体间事件。");
    map.insert(TranslationKey::CommsEventMsg, "消息");
    map.insert(TranslationKey::CommsEventSpawned, "创建");
    map.insert(TranslationKey::CommsEventKilled, "终止");
    map.insert(TranslationKey::CommsEventTask, "任务+");
    map.insert(TranslationKey::CommsEventClaim, "认领");
    map.insert(TranslationKey::CommsEventDone, "完成");
    map.insert(TranslationKey::CommsSendTitle, "发送消息");
    map.insert(TranslationKey::CommsSendFrom, "发送方 (智能体ID):");
    map.insert(TranslationKey::CommsSendTo, "接收方 (智能体ID):");
    map.insert(TranslationKey::CommsSendMsg, "消息:");
    map.insert(TranslationKey::CommsPostTitle, "发布任务");
    map.insert(TranslationKey::CommsPostTitleLabel, "标题:");
    map.insert(TranslationKey::CommsPostDesc, "描述:");
    map.insert(TranslationKey::CommsPostAssign, "分配给 (智能体ID, 可选):");
    map.insert(TranslationKey::CommsHintNavigate, "导航");
    map.insert(TranslationKey::CommsHintSend, "发送");
    map.insert(TranslationKey::CommsHintPost, "发布");
    map.insert(TranslationKey::CommsHintRefresh, "刷新");

    // Security
    map.insert(TranslationKey::SecurityTitle, "安全");
    map.insert(TranslationKey::SecurityFeaturesTitle, "安全功能");
    map.insert(TranslationKey::SecurityHeaderFeature, "功能");
    map.insert(TranslationKey::SecurityHeaderStatus, "状态");
    map.insert(TranslationKey::SecurityHeaderDesc, "描述");
    map.insert(TranslationKey::SecurityInputValidation, "输入验证");
    map.insert(TranslationKey::SecurityRateLimiting, "速率限制");
    map.insert(TranslationKey::SecurityToolSandbox, "工具沙箱");
    map.insert(TranslationKey::SecurityAuditLogging, "审计日志");
    map.insert(TranslationKey::SecurityMessageSigning, "消息签名");
    map.insert(TranslationKey::SecurityEncryptedStorage, "加密存储");
    map.insert(TranslationKey::SecurityStatusActive, "启用");
    map.insert(TranslationKey::SecurityStatusInactive, "禁用");
    map.insert(TranslationKey::SecurityChainTitle, "链验证 ({0} 链)");
    map.insert(TranslationKey::SecurityChainLinks, "({0} 链)");
    map.insert(TranslationKey::SecurityChainOk, "[正常]");
    map.insert(TranslationKey::SecurityChainErr, "[错误]");
    map.insert(TranslationKey::SecurityChainResultTitle, "链验证结果");
    map.insert(TranslationKey::SecurityChainVerified, "所有链验证通过。哈希链完整性确认。");
    map.insert(TranslationKey::SecurityHintVerify, "验证链");
    map.insert(TranslationKey::SecurityHintRefresh, "刷新");
    map.insert(TranslationKey::SecurityHintNavigate, "导航");

    // Audit
    map.insert(TranslationKey::AuditTitle, "审计");
    map.insert(TranslationKey::AuditFilterAll, "全部");
    map.insert(TranslationKey::AuditFilterTool, "工具");
    map.insert(TranslationKey::AuditFilterAgent, "智能体");
    map.insert(TranslationKey::AuditFilterMessage, "消息");
    map.insert(TranslationKey::AuditFilterConfig, "配置");
    map.insert(TranslationKey::AuditFilterSystem, "系统");
    map.insert(TranslationKey::AuditHeaderTime, "时间");
    map.insert(TranslationKey::AuditHeaderAgent, "智能体");
    map.insert(TranslationKey::AuditHeaderAction, "操作");
    map.insert(TranslationKey::AuditHeaderTarget, "目标");
    map.insert(TranslationKey::AuditHeaderChain, "链");
    map.insert(TranslationKey::AuditHeaderHash, "哈希");
    map.insert(TranslationKey::AuditLoading, "加载审计日志...");
    map.insert(TranslationKey::AuditEmpty, "此筛选条件无审计记录。");
    map.insert(TranslationKey::AuditSearchPrompt, "搜索:");
    map.insert(TranslationKey::AuditSearchHint, "输入以搜索智能体、操作或目标...");
    map.insert(TranslationKey::AuditDetailTitle, "审计记录: {0}");
    map.insert(TranslationKey::AuditDetailTimestamp, "时间戳:");
    map.insert(TranslationKey::AuditDetailAgent, "智能体:");
    map.insert(TranslationKey::AuditDetailAction, "操作:");
    map.insert(TranslationKey::AuditDetailTarget, "目标:");
    map.insert(TranslationKey::AuditDetailHash, "哈希:");
    map.insert(TranslationKey::AuditDetailChain, "链:");
    map.insert(TranslationKey::AuditChainValid, "链有效");
    map.insert(TranslationKey::AuditChainInvalid, "链无效");
    map.insert(TranslationKey::AuditChainResultTitle, "链验证");
    map.insert(TranslationKey::AuditChainVerified, "哈希验证通过。链完整性确认。");
    map.insert(TranslationKey::AuditChainFailed, "哈希验证失败。链可能损坏。");
    map.insert(TranslationKey::AuditHintFilter, "筛选");
    map.insert(TranslationKey::AuditHintSearch, "搜索");
    map.insert(TranslationKey::AuditHintRefresh, "刷新");
    map.insert(TranslationKey::AuditHintBack, "返回");

    // Usage
    map.insert(TranslationKey::UsageTitle, "用量");
    map.insert(TranslationKey::UsageTabSummary, "概览");
    map.insert(TranslationKey::UsageTabByModel, "按模型");
    map.insert(TranslationKey::UsageTabByAgent, "按智能体");
    map.insert(TranslationKey::UsageCardRequests, "请求");
    map.insert(TranslationKey::UsageCardTokens, "令牌");
    map.insert(TranslationKey::UsageCardCost, "费用");
    map.insert(TranslationKey::UsageCardLatency, "延迟");
    map.insert(TranslationKey::UsageHeaderModel, "模型");
    map.insert(TranslationKey::UsageHeaderProvider, "服务商");
    map.insert(TranslationKey::UsageHeaderRequests, "请求");
    map.insert(TranslationKey::UsageHeaderTokensIn, "输入令牌");
    map.insert(TranslationKey::UsageHeaderTokensOut, "输出令牌");
    map.insert(TranslationKey::UsageHeaderCost, "费用");
    map.insert(TranslationKey::UsageHeaderAgent, "智能体");
    map.insert(TranslationKey::UsageAutoRefresh, "(每10秒自动刷新)");
    map.insert(TranslationKey::UsageHintRefresh, "刷新");
    map.insert(TranslationKey::UsageHintTab, "标签");
    map.insert(TranslationKey::UsageHintBack, "返回");

    // Logs
    map.insert(TranslationKey::LogsTitle, "日志");
    map.insert(TranslationKey::LogsLevel, "级别:");
    map.insert(TranslationKey::LogsLevelAll, "全部");
    map.insert(TranslationKey::LogsLevelError, "错误");
    map.insert(TranslationKey::LogsLevelWarn, "警告");
    map.insert(TranslationKey::LogsLevelInfo, "信息");
    map.insert(TranslationKey::LogsLevelDebug, "调试");
    map.insert(TranslationKey::LogsLevelTrace, "跟踪");
    map.insert(TranslationKey::LogsAuto, "[自动]");
    map.insert(TranslationKey::LogsManual, "[手动]");
    map.insert(TranslationKey::LogsEntries, "条");
    map.insert(TranslationKey::LogsLoading, "加载日志...");
    map.insert(TranslationKey::LogsEmpty, "此筛选级别无日志。");
    map.insert(TranslationKey::LogsSearchPrompt, "搜索:");
    map.insert(TranslationKey::LogsSearchHint, "输入以搜索来源或消息...");
    map.insert(TranslationKey::LogsHeaderTime, "时间");
    map.insert(TranslationKey::LogsHeaderLevel, "级别");
    map.insert(TranslationKey::LogsHeaderSource, "来源");
    map.insert(TranslationKey::LogsHeaderMsg, "消息");
    map.insert(TranslationKey::LogsHintFilter, "筛选");
    map.insert(TranslationKey::LogsHintAuto, "自动刷新");
    map.insert(TranslationKey::LogsHintSearch, "搜索");
    map.insert(TranslationKey::LogsHintRefresh, "刷新");
    map.insert(TranslationKey::LogsHintScroll, "滚动");
    map.insert(TranslationKey::LogsHintBack, "返回");
    map.insert(TranslationKey::LogsHintSearchMode, "搜索  取消");

    // Placeholder
    map.insert(TranslationKey::PlaceholderTitle, "即将推出");
    map.insert(TranslationKey::PlaceholderMsg, "功能未实现");
    map.insert(TranslationKey::PlaceholderHintNavigate, "导航  返回  Tab 切换标签");
    map.insert(TranslationKey::PlaceholderHintBack, "返回");
    map.insert(TranslationKey::PlaceholderHintTab, "标签");

    // Wizard
    map.insert(TranslationKey::WizardTitle, "设置向导");
    map.insert(TranslationKey::WizardStepWelcome, "欢迎");
    map.insert(TranslationKey::WizardStepProvider, "选择服务商");
    map.insert(TranslationKey::WizardStepApiKey, "配置 API 密钥");
    map.insert(TranslationKey::WizardStepModel, "选择模型");
    map.insert(TranslationKey::WizardStepChannel, "配置通道");
    map.insert(TranslationKey::WizardStepAgent, "创建智能体");
    map.insert(TranslationKey::WizardStepSummary, "配置摘要");
    map.insert(TranslationKey::WizardStepComplete, "完成");
    map.insert(TranslationKey::WizardWelcomeMsg, "让我们设置您的 AgentDiVA 环境。");
    map.insert(TranslationKey::WizardWelcomeDesc, "此向导将引导您配置服务商、通道和第一个智能体。");
    map.insert(TranslationKey::WizardWelcomeSteps, "步骤: 服务商 \u{2192} API密钥 \u{2192} 模型 \u{2192} 通道 \u{2192} 智能体");
    map.insert(TranslationKey::WizardPlaceholderNote, "注意: 这是演示向导，使用占位符数据。");
    map.insert(TranslationKey::WizardHintStart, "[Enter] 开始设置  [Esc] 退出");
    map.insert(TranslationKey::WizardProviderPrompt, "选择要配置的 LLM 服务商:");
    map.insert(TranslationKey::WizardProviderHint, "[\u{2191}\u{2193}] 导航  [Enter] 选择  [s] 跳过  [Esc] 返回");
    map.insert(TranslationKey::WizardApiKeyProvider, "配置: ");
    map.insert(TranslationKey::WizardApiKeyPrompt, "输入您的 API 密钥:");
    map.insert(TranslationKey::WizardApiKeyHint, "[v] 切换可见  [t] 测试  [Enter] 下一步  [s] 跳过  [Esc] 返回");
    map.insert(TranslationKey::WizardModelPrompt, "选择要启用的模型:");
    map.insert(TranslationKey::WizardModelHint, "[Space] 切换  [a] 全选  [n] 全不选  [Enter] 下一步  [s] 跳过  [Esc] 返回");
    map.insert(TranslationKey::WizardChannelPrompt, "选择要配置的消息通道:");
    map.insert(TranslationKey::WizardChannelHint, "[\u{2191}\u{2193}] 导航  [Enter] 设置  [s] 跳过  [Esc] 返回");
    map.insert(TranslationKey::WizardChannelSetupTitle, "设置: ");
    map.insert(TranslationKey::WizardChannelInputHint, "[Enter] 保存  [Esc] 取消");
    map.insert(TranslationKey::WizardAgentPrompt, "创建您的第一个智能体:");
    map.insert(TranslationKey::WizardAgentHint, "[\u{2191}\u{2193}] 模板  [c] 自定义  [Enter] 下一步  [s] 跳过  [Esc] 返回");
    map.insert(TranslationKey::WizardAgentHintCustom, "[输入] 名称  [Enter] 下一步  [Esc] 模板列表");
    map.insert(TranslationKey::WizardSummaryTitle, "配置摘要");
    map.insert(TranslationKey::WizardSummaryNote, "配置将保存为占位符数据。");
    map.insert(TranslationKey::WizardSummaryHint, "[Enter] 完成  [b] 返回编辑  [Esc] 取消");
    map.insert(TranslationKey::WizardCompleteTitle, "设置完成!");
    map.insert(TranslationKey::WizardCompleteSuccess, "配置已成功保存!");
    map.insert(TranslationKey::WizardCompleteMsg, "您的 AgentDiVA 环境已就绪。");
    map.insert(TranslationKey::WizardCompleteHint, "[Enter] 进入主界面  [Esc] 查看");
    map.insert(TranslationKey::WizardContent, "这是占位符向导屏幕。");
    map.insert(TranslationKey::WizardHint, "[Esc] 返回欢迎页");

    // Tab bar
    map.insert(TranslationKey::TabBarCtrlCQuit, "再次按 Ctrl+C 退出");
    map.insert(TranslationKey::TabBarCtrlCHint, "Ctrl+Cx2 退出");
    map.insert(TranslationKey::TabBarTabHint, "Tab/Ctrl+ 切换");

    // Common
    map.insert(TranslationKey::CommonLoading, "加载中...");
    map.insert(TranslationKey::CommonEmpty, "无数据。");
    map.insert(TranslationKey::CommonPlaceholder, "placeholder");
    map.insert(TranslationKey::CommonMock, "(模拟)");
    map.insert(TranslationKey::CommonBack, "返回");
    map.insert(TranslationKey::CommonCancel, "取消");
    map.insert(TranslationKey::CommonSave, "保存");
    map.insert(TranslationKey::CommonConfirm, "确认");
    map.insert(TranslationKey::CommonYes, "是");
    map.insert(TranslationKey::CommonNo, "否");
    map.insert(TranslationKey::CommonSuccess, "成功");
    map.insert(TranslationKey::CommonError, "错误");
    map.insert(TranslationKey::CommonPlaceholderAction, "(placeholder)");

    map
}