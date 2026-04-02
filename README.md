# Claude Buddy Changer

Rust-based local tool for searching, previewing, applying, and restoring Claude Code buddy salts.

## 中文

主要能力：

- 自动读取本机 Claude 配置中的 `userId`
- 根据 `userId + salt` 计算 buddy
- 渲染 ASCII buddy 预览
- 按 species / rarity / eye / hat / shiny / min stat 搜索匹配 salt
- 检测本机 Claude Code binary
- 将选中的 salt 应用到本机 Claude Code
- 恢复原始 salt

### 环境要求

- Rust toolchain
- 本机已安装 Claude Code
- 建议系统中可以直接执行 `claude`

### 启动网页

```bash
cargo build --bins
cargo run --bin server
```

打开：

```text
http://127.0.0.1:43123
```

### CLI 示例

预览当前 buddy：

```bash
cargo run --bin buddy-lab -- preview --user-id YOUR_USER_ID --salt friend-2026-401
```

按条件搜索：

```bash
cargo run --bin buddy-lab -- search --user-id YOUR_USER_ID --species owl --total 500000
```

常用参数：

- `--user-id <id>`
- `--salt <salt>`
- `--species <name>`
- `--rarity <tier>`
- `--eye <glyph>`
- `--hat <name>`
- `--shiny`
- `--min-stat <NAME:value>`
- `--total <n>`
- `--salt-prefix <prefix>`
- `--length <n>`

### 测试

```bash
cargo test
```

测试内容包括：

- buddy 生成与渲染 parity
- HTTP 路由 contract
- binary patch / restore 行为
- 前后端约定的黄金夹具回归

### 项目结构

```text
.
├─ src/
│  ├─ bin/
│  │  ├─ server.rs       # Rust HTTP server entrypoint
│  │  └─ buddy-lab.rs    # Rust CLI entrypoint
│  ├─ buddy.rs           # buddy roll / render / search logic
│  ├─ binary_patch.rs    # Claude binary detect / apply / restore
│  ├─ state.rs           # original salt state file management
│  ├─ web.rs             # HTTP routing and static shell serving
│  ├─ assets.rs          # generated asset fixture loading
│  └─ lib.rs
├─ index.html            # redesigned frontend shell
├─ fixtures/             # golden fixtures used by Rust tests
├─ tests/                # parity and HTTP contract tests
└─ Cargo.toml
```

### 注意事项

- `Use This Buddy` 会修改你本机的 Claude Code 安装文件
- Claude Code 更新后可能覆盖修改
- 应用或恢复后通常需要重启 Claude Code
- 当前前端功能与 Rust 后端保持一致，但 UI 已重设计

## English

This repository is now fully Rust-based for its runtime paths. The legacy JavaScript backend and CLI files have been removed.

Current components:

- Rust HTTP server
- Rust CLI
- redesigned frontend `index.html`
- Rust parity tests and golden fixtures

### Requirements

- Rust toolchain
- local Claude Code installation
- `claude` available on the machine is recommended

### Run the web app

```bash
cargo build --bins
cargo run --bin server
```

Open:

```text
http://127.0.0.1:43123
```

### CLI examples

```bash
cargo run --bin buddy-lab -- preview --user-id YOUR_USER_ID --salt friend-2026-401
cargo run --bin buddy-lab -- search --user-id YOUR_USER_ID --species owl --total 500000
```

### Test

```bash
cargo test
```

### Notes

- buddy results depend on `userId + salt`, not on `salt` alone
- applying a buddy modifies the local Claude Code binary
- restart Claude Code after apply or restore
- the frontend was redesigned, but the feature set and backend contract were kept intact
