Skill Garden CLI v{version} ({os}/{arch})

1. 下载并解压：
   curl -L -o skill-garden-cli.tar.gz "{url}"
   tar xzf skill-garden-cli.tar.gz

2. 安装：
   {install}

3. 验证：
   {verify}

压缩包内容：
- {filename} — CLI 可执行文件
- config.toml — 已预填 API Key（写入 ~/.skill-garden/config.toml）
- install.sh / install.ps1 — 安装脚本（自动复制到程序目录并加入 PATH）
- sill-garden/SKILL.md — 本地 Skill 定义（复制到 Agent 管理的 Skill 目录即可被发现和使用）
