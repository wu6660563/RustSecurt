# FileHide

FileHide 是一个面向 Windows 10/11 的轻量级文件快速隐藏工具。它适合把不希望在日常资源管理器浏览中出现的文件或文件夹暂时隐藏起来。

## 功能说明

- **快速锁定文件**：保存原始 Windows 文件属性，然后设置 `Hidden + System` 属性。
- **快速锁定文件夹**：只修改所选文件夹本身，不递归处理内部文件，适合大文件夹快速生效。
- **恢复原始属性**：从历史记录恢复锁定前的完整属性，而不是简单执行 `attrib -h -s`。
- **状态识别**：显示已锁定、已恢复、路径失效、已被外部解除等状态。
- **历史记录**：使用本机 SQLite 保存路径、类型、原始属性和操作时间。
- **NTFS 找回**：新锁定的项目会写入 `:FileHide` 恢复标记。数据库丢失后，可以在“设置 → 找回锁定项目”中选择目录扫描并逐项恢复。
- **可选访问密码**：在“设置”中启用后，每次启动 FileHide 都需要输入密码。密码使用 Argon2id 哈希保存，不保存明文。
- **WebView2 检查**：启动时检测 Microsoft Edge WebView2 Runtime。缺失时弹出 Windows 原生提示，并可打开微软官方下载页面。
- **批量与拖拽锁定**：文件/文件夹支持多选，亦可从资源管理器直接拖入批量锁定。
- **路径变化识别**：保存 NTFS 卷序列号与文件索引组成的目标身份；原路径对应目标被替换时显示“路径已变化”，避免误恢复。
- **会话自动锁定**：访问密码启用后支持关闭、5/15/30/60 分钟自动锁定。
- **锁定前风险预览**：锁定前检查路径、类型、重复记录、恢复标记和 Reparse Point，并在有风险时要求确认。
- **批量恢复**：可勾选多个项目逐项恢复，单项失败不会影响其他项目。
- **启动健康检查**：启动时检查文件身份、隐藏属性和路径状态，区分移动、替换、删除与外部解除隐藏。
- **异常事务恢复**：锁定操作写入本地事务日志；程序异常退出后启动时清理残留日志并保留可疑记录，避免误删锁定历史。
- **便携版模式**：在 `filehide.exe` 同目录创建空文件 `portable.flag` 后，数据库会保存到 EXE 同目录，适合 U 盘携带；不创建该文件则继续使用 Windows 应用数据目录。

## 重要安全边界

FileHide 的快速锁定不是加密，也不是操作系统级访问控制。

它不会：

- 读取、修改或加密文件内容；
- 移动或压缩文件；
- 递归修改文件夹内部项目；
- 阻止管理员或熟悉 Windows 的用户通过显示隐藏项目、命令行、其他工具或直接修改属性来访问文件。

因此不要把它用于密码、私钥、商业机密等必须防止技术人员访问的数据。需要真正的机密性时，应使用 BitLocker、加密磁盘或后续的加密保险箱方案。

## 运行要求

- Windows 10 1803 及以上或 Windows 11；
- Microsoft Edge WebView2 Runtime；
- 文件所在磁盘使用 NTFS 时，才能使用 ADS 找回标记功能。FAT/exFAT 仍可进行普通属性隐藏，但不支持数据库丢失后的标记扫描；
- 不需要单独安装 Node.js 或 Rust 才能运行编译后的 EXE。

## 开发环境

首次开发需要安装：

- Rust stable（MSVC 工具链）；
- Node.js 18 或更高版本；
- Windows 的 Tauri 开发依赖和 WebView2 Runtime。

安装前端依赖：

```powershell
npm install
```

启动开发模式：

```powershell
npm run tauri dev
```

运行前端测试：

```powershell
npm test
```

运行 Rust 测试：

```powershell
cargo test --manifest-path src-tauri/Cargo.toml
```

## 编译发布版

在项目根目录执行：

```powershell
npm run tauri build
```

编译产物位于：

```text
src-tauri\target\release\filehide.exe
```

当前项目关闭了 Tauri 安装包生成（`bundle.active: false`），因此发布给别人时可以直接把 `filehide.exe` 上传到 GitHub Release。仓库中的 `target/`、`dist/`、`node_modules/`、EXE 和调试符号已由 `.gitignore` 排除。

### 发布到 GitHub Release

```powershell
git add .
git commit -m "feat: initial FileHide release"
git branch -M main
git remote add origin https://github.com/<your-name>/<your-repository>.git
git push -u origin main
```

然后在 GitHub 仓库的 **Releases → Draft a new release** 中创建版本，例如 `v1.1.0`，将下面的文件作为 Release asset 上传：

```text
src-tauri\target\release\filehide.exe
```

不需要上传 `filehide.pdb`、`target` 文件夹或 `node_modules`。

## 使用方法

### 锁定文件或文件夹

1. 启动 FileHide。
2. 点击“锁定文件”或“锁定文件夹”。
3. 在 Windows 选择器中选择目标。
4. FileHide 保存原始属性并立即设置隐藏和系统属性。
5. 在“隐藏项目”中查看实时状态。

锁定文件夹时不会扫描子目录，因此速度主要取决于一次 Windows 属性写入，而不是文件夹内的文件数量。

### 恢复项目

在“隐藏项目”或“历史记录”中点击“恢复”。只有仍处于 FileHide 锁定状态的项目会显示恢复按钮；如果用户已在资源管理器中手动取消隐藏，FileHide 会标记为“已外部解除”。

### 设置访问密码

1. 打开“设置”。
2. 输入新密码和确认密码，点击“启用访问密码”。
3. 下次启动时输入密码才能访问 FileHide。
4. 修改或关闭密码时需要输入当前密码。

请妥善保存密码。当前版本没有密码找回按钮；忘记密码时不要直接删除数据库，先保留带 ADS 标记的文件并使用“找回锁定项目”功能。

### 数据库丢失后的找回

1. 打开 FileHide 的“设置”。
2. 点击“选择目录并扫描”。
3. 选择一个大致范围，而不是整个磁盘。
4. 等待扫描完成，可随时取消。
5. 对找到的项目逐项点击“找回并恢复”。

扫描只读取目录元数据和很小的 `:FileHide` 标记，不读取文件内容，也不会自动恢复项目。扫描时间取决于所选目录的文件数量、磁盘速度和网络/移动磁盘状态。

## 数据位置与重装

FileHide 会在当前用户的 Tauri 应用数据目录中创建 `filehide.db`。普通卸载或删除 EXE 通常不会自动删除该数据库，因此重新安装同一应用标识后可以继续看到历史记录。

如果应用数据也被清理，旧项目只能通过 NTFS `:FileHide` 标记找回；没有新标记的旧版本锁定项目无法在全盘中可靠区分出来。

## 项目结构

```text
.
├── src/                         # Vue 3 + TypeScript 前端
├── src-tauri/src/               # Rust/Tauri 后端
├── src-tauri/Cargo.toml         # Rust 依赖
├── src-tauri/tauri.conf.json    # Tauri 构建和安全配置
├── docs/                        # 设计、审查和实现计划
├── package.json                 # 前端脚本
└── .gitignore                   # Git 排除规则
```

## 许可证

当前项目尚未指定开源许可证。公开到 GitHub 前，请根据你的分发方式补充合适的 `LICENSE` 文件。
