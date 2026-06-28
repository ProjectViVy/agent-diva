//! Pre-defined named toolset groups for composing tool configurations.

#[derive(Debug, Clone)]
pub struct Toolset {
    pub name: &'static str,
    pub description: &'static str,
    pub tools: &'static [&'static str],
}

pub const CORE_TOOLSET: Toolset = Toolset {
    name: "core", description: "核心工具集",
    tools: &["read_file","write_file","patch","search_files","terminal","web_search","web_extract"],
};
pub const FILE_TOOLSET: Toolset = Toolset {
    name: "file", description: "文件操作",
    tools: &["read_file","write_file","patch","search_files","list_dir"],
};
pub const SHELL_TOOLSET: Toolset = Toolset {
    name: "shell", description: "命令执行",
    tools: &["terminal","process"],
};
pub const WEB_TOOLSET: Toolset = Toolset {
    name: "web", description: "网络搜索",
    tools: &["web_search","web_extract"],
};
pub const BROWSER_TOOLSET: Toolset = Toolset {
    name: "browser", description: "浏览器自动化（Phase 3）",
    tools: &["browser_navigate","browser_snapshot","browser_click","browser_type","browser_scroll"],
};
pub const CODE_TOOLSET: Toolset = Toolset {
    name: "code", description: "代码执行与委托（Phase 2）",
    tools: &["execute_code","delegate_task"],
};

pub const ALL_TOOLSETS: &[Toolset] = &[
    CORE_TOOLSET, FILE_TOOLSET, SHELL_TOOLSET, WEB_TOOLSET, BROWSER_TOOLSET, CODE_TOOLSET,
];

pub fn find_toolset(name: &str) -> Option<&'static Toolset> {
    ALL_TOOLSETS.iter().find(|ts| ts.name == name)
}
