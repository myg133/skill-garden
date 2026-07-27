//! CLI 命令实现

use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;

use super::client::{ApiClient, ListResult, SkillDetail};
use super::config::CliConfig;

/// 登录：验证 API Key + 保存配置
pub async fn login(client: &ApiClient, server: &str, token: &str) -> Result<()> {
    // 调用 skills.list 验证 API Key 有效性
    client.verify_auth().await?;
    println!("认证成功!");
    println!("  服务端: {}", server);

    // 保存配置
    let mut config = CliConfig::load()?;
    config.server = Some(server.to_string());
    config.token = Some(token.to_string());
    config.save()?;
    println!("\n配置已保存到 ~/.skill-garden/config.toml");

    Ok(())
}

/// 登出：删除配置文件
pub fn logout() -> Result<()> {
    CliConfig::delete()?;
    println!("已登出，配置文件已删除");
    Ok(())
}

/// 查看当前身份
pub async fn whoami(client: &ApiClient) -> Result<()> {
    match client.session_info().await {
        Ok(info) => {
            println!("已登录:");
            println!("  会话 ID:   {}", info.session_id);
            println!("  身份 ID:   {}", info.identity_id);
            println!("  组织 ID:   {}", info.org_id);
            println!("  会话状态: {}", info.status);
        }
        Err(e) => {
            // session.info 可能失败，降级为验证 API Key
            match client.verify_auth().await {
                Ok(_) => {
                    println!("已登录（API Key 有效）");
                    println!("  服务端: {}", client.server);
                }
                Err(_) => {
                    anyhow::bail!("未登录或 token 无效: {}", e);
                }
            }
        }
    }
    Ok(())
}

/// 搜索 Skills
pub async fn search(client: &ApiClient, query: &str, limit: u32) -> Result<()> {
    let results = client.search(query, limit).await?;
    if results.is_empty() {
        println!("没有找到匹配的 Skill");
        return Ok(());
    }

    println!("{:<40} {:<12} {:<10}", "SKILL ID", "VERSION", "SCORE");
    println!("{}", "-".repeat(66));
    for r in &results {
        let id = r.skill_id.as_deref().unwrap_or("-");
        let ver = r.version.as_deref().unwrap_or("-");
        let score = r.score.map(|s| format!("{:.2}", s)).unwrap_or_default();
        println!("{:<40} {:<12} {:<10}", truncate(id, 38), ver, score);
        if let Some(desc) = &r.description {
            if !desc.is_empty() {
                println!("  {}", desc);
            }
        }
    }
    println!("\n共 {} 条结果", results.len());
    Ok(())
}

/// 列出 Skills
pub async fn list(client: &ApiClient, page: u32, page_size: u32) -> Result<()> {
    let offset = (page.saturating_sub(1)) * page_size;
    let resp = client.list(page_size, offset, "created").await?;
    print_list_result(resp, page);
    Ok(())
}

/// 查看 Skill 详情
pub async fn info(client: &ApiClient, skill_id: &str) -> Result<()> {
    let detail = client.info(skill_id).await?;
    print_skill_detail(&detail);
    Ok(())
}

/// 安装 Skill（下载到本地）
pub async fn install(
    client: &ApiClient,
    skill_id: &str,
    target_dir: Option<&str>,
    config_skills_dir: Option<&str>,
) -> Result<()> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::with_template("{spinner} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    spinner.set_message(format!("正在获取 {} 的安装信息...", skill_id));
    spinner.enable_steady_tick(std::time::Duration::from_millis(80));

    let install_result = client.install(skill_id).await?;

    spinner.finish_and_clear();

    let download_url = install_result
        .download_url
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("服务端未返回下载链接"))?;

    println!("Skill: {} v{}", install_result.name, install_result.version);
    println!("描述:   {}", install_result.description);
    println!("作者:   {}", install_result.author_agent_id);
    println!("文件数: {}", install_result.file_count);

    // 确定 Skills 根目录：--dir > config.skills_dir > cwd/skills。
    // tarball 自带原始 Skill 文件夹，CLI 不再额外追加 Skill 名称。
    let dest_dir = match target_dir {
        Some(d) => PathBuf::from(d),
        None => match config_skills_dir {
            Some(d) => PathBuf::from(d),
            None => {
                let cwd = std::env::current_dir()?;
                cwd.join("skills")
            }
        },
    };

    // 下载并解压
    let pb = ProgressBar::new(install_result.tarball_size);
    pb.set_style(
        ProgressStyle::with_template("{spinner} 下载中... {bytes}/{total_bytes} ({bytes_per_sec})")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb.enable_steady_tick(std::time::Duration::from_millis(80));

    client
        .download_tarball(download_url, &dest_dir.to_string_lossy())
        .await?;

    pb.finish_with_message("下载完成");

    println!("\nSkill 已安装到: {}", dest_dir.display());
    if !install_result.install_hint.is_empty() {
        println!("\n提示: {}", install_result.install_hint);
    }

    Ok(())
}

/// 列出 Skill 的历史版本
pub async fn versions(client: &ApiClient, skill_name: &str) -> Result<()> {
    let vers = client.versions(skill_name).await?;

    if vers.is_empty() {
        println!("未找到 {} 的版本信息", skill_name);
        return Ok(());
    }

    println!("{} 的版本历史:\n", skill_name);
    println!("{:<16} {:<40} {:<12}", "VERSION", "SKILL ID", "STATUS");
    println!("{}", "-".repeat(72));
    for v in &vers {
        println!("{:<16} {:<40} {:<12}", v.version, v.skill_id, v.status);
    }
    println!("\n共 {} 个版本", vers.len());

    Ok(())
}

/// 热门技能
pub async fn popular(client: &ApiClient, limit: u32) -> Result<()> {
    let items = client.popular(limit).await?;
    if items.is_empty() {
        println!("没有找到热门 Skill");
        return Ok(());
    }

    println!(
        "{:<40} {:<12} {:<10} {:<10}",
        "ID", "VERSION", "INSTALLS", "STATUS"
    );
    println!("{}", "-".repeat(76));
    for s in &items {
        println!(
            "{:<40} {:<12} {:<10} {:<10}",
            truncate(&s.id, 38),
            s.version,
            s.install_count,
            s.status,
        );
        if !s.description.is_empty() {
            println!("  {}", s.description);
        }
    }
    println!("\n共 {} 个热门 Skill", items.len());
    Ok(())
}

/// 技能统计
pub async fn stats(client: &ApiClient, skill_id: &str) -> Result<()> {
    let s = client.stats(skill_id).await?;

    println!("Skill {} 使用统计:\n", skill_id);
    println!("  总执行次数: {}", s.total_evaluations);
    println!("  成功次数:   {}", s.success_count);
    println!("  失败次数:   {}", s.failure_count);
    println!("  成功率:     {:.1}%", s.success_rate * 100.0);
    println!("  平均耗时:   {:.1}ms", s.avg_duration_ms);
    if !s.confidence.is_empty() {
        println!("  置信度:     {}", s.confidence);
    }
    if !s.tags.is_empty() {
        println!("  标签:       {}", s.tags.join(", "));
    }

    Ok(())
}

/// 显示配置
pub fn config_show() -> Result<()> {
    let config = CliConfig::load()?;
    println!("配置文件: ~/.skill-garden/config.toml\n");
    println!(
        "server     = {:?}",
        config.server.as_deref().unwrap_or("未设置")
    );
    println!("token      = {:?}", mask_token(config.token.as_deref()));
    println!(
        "skills_dir = {:?}",
        config
            .skills_dir
            .as_deref()
            .unwrap_or("未设置（默认 ./skills/）")
    );
    Ok(())
}

/// 设置配置项
pub fn config_set(key: &str, value: &str) -> Result<()> {
    let mut config = CliConfig::load()?;
    match key {
        "server" => config.server = Some(value.to_string()),
        "token" => config.token = Some(value.to_string()),
        "skills_dir" => config.skills_dir = Some(value.to_string()),
        _ => anyhow::bail!("未知配置项: {}。支持: server, token, skills_dir", key),
    }
    config.save()?;
    println!(
        "配置已更新: {} = {}",
        key,
        if key == "token" { "***" } else { value }
    );
    Ok(())
}

// ========== helpers ==========

fn print_list_result(resp: ListResult, page: u32) {
    let skills = &resp.skills;
    if skills.is_empty() {
        println!("没有找到 Skill");
        return;
    }

    println!(
        "{:<40} {:<12} {:<10} {:<10}",
        "ID", "VERSION", "INSTALLS", "STATUS"
    );
    println!("{}", "-".repeat(76));
    for s in skills {
        println!(
            "{:<40} {:<12} {:<10} {:<10}",
            truncate(&s.id, 38),
            s.version,
            s.install_count,
            s.status,
        );
    }
    println!("\n第 {} 页，共 {} 个 Skill", page, resp.total);
}

fn print_skill_detail(detail: &SkillDetail) {
    println!("ID:          {}", detail.id);
    println!("名称:        {}", detail.name);
    println!("版本:        {}", detail.version);
    println!("状态:        {}", detail.status);
    if !detail.visibility.is_empty() {
        println!("可见性:      {}", detail.visibility);
    }
    println!("安装次数:    {}", detail.install_count);
    println!("作者:        {}", detail.author_agent_id);
    if !detail.tags.is_empty() {
        println!("标签:        {}", detail.tags.join(", "));
    }
    if !detail.dependencies.is_empty() {
        println!("依赖:        {}", detail.dependencies.join(", "));
    }
    if !detail.tools.is_empty() {
        println!("工具:        {}", detail.tools.join(", "));
    }
    println!("创建时间:    {}", detail.created);
    println!("更新时间:    {}", detail.updated);
    println!();
    println!("描述:");
    println!("  {}", detail.description);
    if !detail.content.is_empty() {
        println!();
        // 只显示前 500 个字符的 content
        let preview = truncate(&detail.content, 500);
        let total_chars = detail.content.chars().count();
        println!("内容预览:");
        println!("{}", preview);
        if total_chars > 500 {
            println!("... (共 {} 字符)", total_chars);
        }
    }
}

fn truncate(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        Some((byte_idx, _)) => &s[..byte_idx],
        None => s,
    }
}

fn mask_token(token: Option<&str>) -> &str {
    match token {
        Some(_) => "已设置",
        None => "未设置",
    }
}
