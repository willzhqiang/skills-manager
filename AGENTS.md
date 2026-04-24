# skills-manager repo

本仓库是 `skills-manager` 项目的**代码侧**。项目的规则、索引、对话日志、衍生文档等都在 Obsidian vault：

- **项目根**：`/Users/Qiang/Documents/Obsidian/Projects/Tech/skills-manager/`
- **代码约定**（编辑代码前必读）：`<项目根>/Code-Conventions.md`
- **项目索引**：`<项目根>/Index.md`
- **Q/A 日志**：`<项目根>/Conversations.md`

---

## 如果 Agent 从本仓库启动（非推荐）

正常工作流是**从 vault 的项目根启动 Agent**（用绝对路径调用本仓库的文件）。本仓库不再维护独立的编码规则。

如果出于特殊原因从这里启动（比如临时在 Cursor / Claude Code 里打开代码目录），请按顺序：

1. 读 `/Users/Qiang/Documents/Obsidian/Projects/Tech/skills-manager/Code-Conventions.md`
2. 读 `/Users/Qiang/Documents/Obsidian/Projects/Tech/skills-manager/Index.md`
3. 读 `/Users/Qiang/Documents/Obsidian/Projects/Tech/skills-manager/Conversations.md` 末尾的最后 2 条 Q/A
4. 读 vault 全局规则：`/Users/Qiang/Documents/Obsidian/AGENTS.md`

注意：从代码仓库启动会**失去** vault 的 hook 链路（Q/A 自动捕获与 A 摘要兜底都不生效）。建议调整工作方式，从 vault 项目根启动。
