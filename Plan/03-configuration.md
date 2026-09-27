# 03 · 配置设计

决策：**OAuth 全砍 + 无默认端点** → 配置收敛为「显式 base_url + 显式 key」。

---

## 1. 配置入口（3 个）

### 入口 1：`settings.json`（推荐主入口）

用户级 `~/.claude/settings.json`，或项目级 `<项目>/.claude/settings.json`。

OpenAI 兼容示例：
```jsonc
{
  "modelType": "openai",
  "model": "deepseek-chat",
  "env": {
    "OPENAI_BASE_URL": "https://api.deepseek.com/v1",
    "OPENAI_API_KEY": "sk-xxxx"
  }
}
```

Anthropic 兼容示例：
```jsonc
{
  "modelType": "anthropic",
  "model": "claude-sonnet-4-5",
  "env": {
    "ANTHROPIC_BASE_URL": "https://my-gateway.example.com",
    "ANTHROPIC_AUTH_TOKEN": "sk-xxxx"
  }
}
```

### 入口 2：环境变量（临时 / CI）

```bash
# Anthropic 兼容
export ANTHROPIC_BASE_URL=https://my-gateway.example.com
export ANTHROPIC_API_KEY=sk-xxxx

# OpenAI 兼容
export OPENAI_BASE_URL=https://api.deepseek.com/v1
export OPENAI_API_KEY=sk-xxxx
```
`env` 块本质是"写进 settings 的环境变量"，两者同构。

### 入口 3：`/provider` + `/config`（交互式，写入 settings.json）

- `/login` 删除（OAuth 入口消失）；其"填 base_url + key"职责由 `/provider` 承接
- `/provider`：选协议（anthropic / openai）+ 填 base_url/key → 写入 settings.json
- `/config`：通用设置面板

### 可选：`apiKeyHelper`（动态 key）

```jsonc
{ "apiKeyHelper": "vault read -field=key secret/claude" }
```
适合 key 由密钥管理系统动态下发、不落盘的场景。

---

## 2. 优先级

```
CLI 参数 (--api-key / --base-url)
  > 环境变量 (process.env)
  > 项目 .claude/settings.json 的 env
  > 用户 ~/.claude/settings.json 的 env
  > ❌ 无默认，缺失即报错
```

settings.json 各层叠加顺序（沿用现有语义）：
`user < project < local < policy/managed`（policy/managed 可选砍）。

---

## 3. 缺失配置的错误提示

启动时若两协议均未配置 base_url，必须给出可操作提示：

```
Error: No API endpoint configured.

Configure one of the following:

  Anthropic-compatible:
    export ANTHROPIC_BASE_URL=<url>
    export ANTHROPIC_API_KEY=<key>

  OpenAI-compatible:
    export OPENAI_BASE_URL=<url>
    export OPENAI_API_KEY=<key>

  Or edit ~/.claude/settings.json ("modelType" + "env").
```

---

## 4. 密钥存储

| 数据 | 存储位置 | 是否秘密 |
|---|---|---|
| base_url | `settings.json` 的 `env` | 否 |
| model / modelType | `settings.json` | 否 |
| API Key | macOS Keychain（`keyring` crate）；非 macOS 回落 `~/.claude.json` | 是 |

---

## 5. 需保留 / 删除的配置项

### 保留
- settings.json 多层加载 + 合并（`env` 块合并进运行时）
- `modelType`（**仅 2 值**：`anthropic` / `openai`）
- `model` / `availableModels` / `modelOverrides`
- `apiKeyHelper`
- `permissions`、`hooks`、`env`、`statusLine` 等通用项
- Keychain / `~/.claude.json` 存 key

### 删除
- policy/managed settings（企业 MDM，可选）
- OAuth 分支：`forceLoginMethod`、`forceLoginOrgUUID`
- Provider 分支：`gemini` / `grok` / `bedrock` / `vertex` / `foundry`
- 云凭据刷新：`awsCredentialExport`、`awsAuthRefresh`、`gcpAuthRefresh`
- `xaaIdp`、MCP OAuth 配置
- `/login` OAuth UI、`ConsoleOAuthFlow`
