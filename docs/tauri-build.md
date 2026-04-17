# Tauri 构建注意事项

## 禁止分步构建

Tauri v2 在 Rust 编译阶段通过构建脚本将前端 dist 目录嵌入二进制文件。
分步执行 `cargo build --release` + `npx tauri bundle` 会导致二进制缺少前端资源，打开应用白屏。

### 正确做法

```bash
npx tauri build
```

一步完成：前端构建 → 设置 `TAURI_FRONTEND_DIST` 环境变量 → 编译 Rust（嵌入前端）→ 打包 .app/.dmg

### 错误做法

```bash
npm run build          # 前端构建
cargo build --release  # 缺少 Tauri CLI 设置的环境变量，前端不会被嵌入
npx tauri bundle       # 打包了一个空壳二进制
```

### 签名警告

`npx tauri build` 末尾的 `TAURI_SIGNING_PRIVATE_KEY` 错误可忽略，不影响 .app 和 .dmg 生成。

### 开发模式

```bash
npm run tauri:dev
```

dev 模式下前端由 Vite dev server 提供（`http://127.0.0.1:1420`），不存在嵌入问题。
