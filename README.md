# Notepad

高性能 Markdown 桌面记事本，基于 Rust + Iced GUI 框架 + SQLite 构建。

支持层级文件夹管理、实时 Markdown 预览、全文搜索、图片嵌入、撤销/重做、深色/浅色主题切换等功能。

## 功能特性

### 笔记管理
- 层级文件夹结构，支持无限嵌套
- 文件夹/笔记的创建、重命名、删除（级联删除）
- 笔记导出为 `.md` 文件

### Markdown 编辑
- 分屏编辑：左侧源码编辑，右侧实时预览
- 全屏阅读模式
- 工具栏快捷插入：标题、加粗、斜体、删除线、代码块、列表、引用、链接、表格等
- 代码块语法高亮
- 编辑/预览滚动同步

### 图片支持
- 支持 PNG、JPG、GIF、WebP、BMP 格式
- 图片以二进制存入 SQLite，无需外部文件依赖
- Markdown 预览中内联渲染图片

### 全文搜索
- 标题 + 内容全局搜索（不区分大小写）
- 搜索结果高亮显示
- 点击结果跳转到匹配位置
- 单笔记搜索支持上下切换命中、准确定位与正文高亮

### 编辑增强
- 撤销/重做（最多 100 步）
- 自动保存（500ms 防抖）
- 可调字体大小（10-28pt）
- 字符数、字数、行数统计及预估阅读时长

### 主题
- 深色主题（Tokyo Night）
- 浅色主题
- 一键切换

## 键盘快捷键

| 快捷键 | 功能 |
|--------|------|
| `Ctrl+N` | 新建笔记 |
| `Ctrl+S` | 保存笔记 |
| `Ctrl+E` | 切换编辑/阅读模式 |
| `Ctrl+F` | 聚焦当前笔记搜索 |
| `Ctrl+Shift+F` | 聚焦全局搜索框 |
| `Ctrl+Z` | 撤销 |
| `Ctrl+Shift+Z` / `Ctrl+Y` | 重做 |
| `Ctrl+=` / `Ctrl+-` | 调整字体大小 |
| `Ctrl+Shift+E` | 导出为 Markdown 文件 |
| `Esc` | 关闭菜单 / 退出编辑模式 |

## 技术栈

| 组件 | 技术 |
|------|------|
| GUI 框架 | [Iced](https://github.com/iced-rs/iced) 0.14 |
| 数据库 | SQLite（WAL 模式，通过 rusqlite） |
| 异步运行时 | Tokio |
| 文件对话框 | rfd |
| 唯一标识 | UUID v4 |

## 构建与运行

### 前置要求

- Rust 工具链（Edition 2024）
- 系统依赖（Linux）：
  ```bash
  # Ubuntu/Debian
  sudo apt install pkg-config libgtk-3-dev libxkbcommon-dev
  ```

### 构建

```bash
# Debug 构建
cargo build

# Release 构建（推荐，启用 LTO 和符号裁剪）
cargo build --release
```

### 运行

```bash
cargo run --release
```

数据库文件 `notepad.db` 会自动创建在可执行文件所在目录。

## 项目结构

```
src/
├── main.rs          # 应用入口
├── message.rs       # 全局消息枚举
├── app/             # 应用状态与更新逻辑
│   ├── mod.rs       # App 结构体、视图、订阅
│   ├── update.rs    # 消息处理
│   └── tree_ops.rs  # 树操作工具
├── db/              # 数据库层
│   ├── connection.rs  # 连接池（单线程后台 worker）
│   ├── migration.rs   # 数据库迁移
│   ├── folder.rs      # 文件夹 CRUD
│   ├── note.rs        # 笔记 CRUD + 全文搜索
│   └── image.rs       # 图片存储
├── model/           # 数据模型
│   ├── folder.rs    # Folder、TreeNode
│   ├── note.rs      # Note、SearchResult
│   └── image.rs     # ImageAttachment
├── ui/              # 界面组件
│   ├── sidebar.rs     # 侧边栏（文件树 + 搜索）
│   ├── editor.rs      # 分屏编辑器
│   ├── toolbar.rs     # 工具栏
│   ├── status_bar.rs  # 状态栏
│   ├── welcome.rs     # 欢迎页
│   ├── md_viewer.rs   # Markdown 渲染（含图片）
│   ├── md_shortcut.rs # Markdown 快捷插入
│   ├── search_highlight.rs # 搜索高亮
│   ├── image_picker.rs    # 图片选择器
│   └── error_banner.rs    # 错误提示
└── util/
    └── id.rs        # UUID 生成
```

## 架构设计

- **异步数据库访问**：通过 mpsc channel 将数据库操作委托给后台线程，UI 线程永不阻塞
- **Elm 架构**：遵循 Iced 的 Model-Update-View 模式，状态管理清晰
- **懒加载**：文件夹子项按需加载，避免启动时全量读取
- **防抖保存**：编辑后 500ms 自动保存，兼顾响应性与磁盘 IO

## 数据库优化

- WAL 模式支持并发读取
- 索引覆盖常用查询路径
- 外键约束 + 级联删除保证数据一致性
- 内存临时存储，8000 页缓存

## 测试

```bash
cargo test
```

包含全文搜索逻辑的单元测试（行列计算、大小写匹配、摘要提取、多匹配检测等）。

## 许可证

[Apache License 2.0](LICENSE)
