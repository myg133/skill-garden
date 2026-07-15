//! Skill Garden CLI — 终端入口
//!
//! ```text
//! skill-garden login    <server>             用 API Key 登录
//! skill-garden logout                        清除本地凭证
//! skill-garden whoami                        查看当前身份
//! skill-garden search   <query>              全文搜索技能
//! skill-garden list     [--page] [--size]    列出技能
//! skill-garden info     <skill-id>           技能详情
//! skill-garden install  <skill-id>           安装技能到本地
//! skill-garden versions <name>               查看版本历史
//! skill-garden popular  [--limit]            热门技能
//! skill-garden stats    <skill-id>           技能统计
//! skill-garden config   [set <key> <val>]    管理配置
//! ```

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "skill-garden", about = "Skill Garden CLI", version)]
struct Cli {
    /// 服务端地址（优先级高于配置文件）
    #[arg(short = 'S', long, global = true)]
    server: Option<String>,

    /// API Key（优先级高于配置文件）
    #[arg(short = 'T', long, global = true)]
    token: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 登录到 Skill Garden 服务端
    Login {
        /// 服务端地址
        server: String,
        /// API Key（sk_ 前缀），不提供则交互输入
        #[arg(short, long)]
        token: Option<String>,
    },
    /// 清除本地登录凭证
    Logout,
    /// 查看当前登录身份
    Whoami,
    /// 搜索 Skill
    Search {
        /// 搜索关键词
        query: String,
        /// 返回条数
        #[arg(short, long, default_value = "20")]
        limit: u32,
    },
    /// 列出 Skill
    List {
        /// 页码（从 1 开始）
        #[arg(short, long, default_value = "1")]
        page: u32,
        /// 每页条数
        #[arg(short = 'n', long, default_value = "20")]
        page_size: u32,
    },
    /// 查看 Skill 详情
    Info {
        /// Skill ID
        skill_id: String,
    },
    /// 安装 Skill 到本地
    Install {
        /// Skill ID
        skill_id: String,
        /// 安装目录（默认 ./skills/<skill-name>/）
        #[arg(short, long)]
        dir: Option<String>,
    },
    /// 查看 Skill 版本历史
    Versions {
        /// Skill 名称
        name: String,
    },
    /// 热门 Skill 排行
    Popular {
        /// 返回条数
        #[arg(short, long, default_value = "20")]
        limit: u32,
    },
    /// 查看 Skill 使用统计
    Stats {
        /// Skill ID
        skill_id: String,
    },
    /// 管理配置
    #[command(subcommand)]
    Config(ConfigCmd),
}

#[derive(Subcommand)]
enum ConfigCmd {
    /// 显示当前配置
    Show,
    /// 设置配置项
    Set {
        /// 配置键：server, token, skills_dir
        key: String,
        /// 配置值
        value: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Login { server, token } => {
            let token = match token {
                Some(t) => t.clone(),
                None => {
                    println!("请输入 API Key（sk_ 前缀）:");
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input)?;
                    input.trim().to_string()
                }
            };
            let client = aion_hive::cli::client::ApiClient::new(server.clone(), token.clone());
            aion_hive::cli::commands::login(&client, server, &token).await?;
        }
        Commands::Logout => {
            aion_hive::cli::commands::logout()?;
        }
        Commands::Whoami => {
            let client = resolve_client(&cli)?;
            aion_hive::cli::commands::whoami(&client).await?;
        }
        Commands::Search { query, limit } => {
            let client = resolve_client(&cli)?;
            aion_hive::cli::commands::search(&client, query, *limit).await?;
        }
        Commands::List { page, page_size } => {
            let client = resolve_client(&cli)?;
            aion_hive::cli::commands::list(&client, *page, *page_size).await?;
        }
        Commands::Info { skill_id } => {
            let client = resolve_client(&cli)?;
            aion_hive::cli::commands::info(&client, skill_id).await?;
        }
        Commands::Install { skill_id, dir } => {
            let config = aion_hive::cli::config::CliConfig::load()?;
            let client = resolve_client_from_config(&config, &cli)?;
            aion_hive::cli::commands::install(
                &client,
                skill_id,
                dir.as_deref(),
                config.skills_dir.as_deref(),
            )
            .await?;
        }
        Commands::Versions { name } => {
            let client = resolve_client(&cli)?;
            aion_hive::cli::commands::versions(&client, name).await?;
        }
        Commands::Popular { limit } => {
            let client = resolve_client(&cli)?;
            aion_hive::cli::commands::popular(&client, *limit).await?;
        }
        Commands::Stats { skill_id } => {
            let client = resolve_client(&cli)?;
            aion_hive::cli::commands::stats(&client, skill_id).await?;
        }
        Commands::Config(cmd) => match cmd {
            ConfigCmd::Show => {
                aion_hive::cli::commands::config_show()?;
            }
            ConfigCmd::Set { key, value } => {
                aion_hive::cli::commands::config_set(key, value)?;
            }
        },
    }

    Ok(())
}

/// 从配置文件 + 命令行参数解析 API 客户端
fn resolve_client(cli: &Cli) -> Result<aion_hive::cli::client::ApiClient> {
    let config = aion_hive::cli::config::CliConfig::load()?;
    resolve_client_from_config(&config, cli)
}

/// 从已加载的配置 + 命令行参数解析 API 客户端
fn resolve_client_from_config(
    config: &aion_hive::cli::config::CliConfig,
    cli: &Cli,
) -> Result<aion_hive::cli::client::ApiClient> {
    let server = cli.server.clone().or_else(|| config.server.clone()).ok_or_else(|| {
        anyhow::anyhow!("未配置服务端地址。请先运行: skill-garden login <server>")
    })?;

    let token =
        cli.token.clone().or_else(|| config.token.clone()).ok_or_else(|| {
            anyhow::anyhow!("未配置 API Key。请先运行: skill-garden login <server>")
        })?;

    Ok(aion_hive::cli::client::ApiClient::new(server, token))
}
