# FileHide 快速隐藏工具 V1.0 详细设计方案

版本：V1.0  
技术路线：Rust + Tauri 2 + Vue3 + TypeScript  
运行平台：Windows 10 / Windows 11

---

# 1. 产品定位

FileHide 是一个轻量级 Windows 文件快速隐藏工具。

V1.0 核心目标：

- 快速隐藏文件
- 快速隐藏文件夹
- 快速恢复显示
- 保存隐藏历史
- 提供现代化 Windows 风格界面

本版本不实现文件加密。

设计原则：

> 不修改文件内容，不移动文件，不压缩文件，仅修改 Windows 文件属性，实现毫秒级隐藏。

---

# 2. 核心设计原则

## 2.1 不递归隐藏

隐藏文件夹时：

只修改目标文件夹本身属性。

例如：

```
D:\私人资料

├── 图片
├── 文档
└── 视频
```

执行隐藏：

```
SetFileAttributesW(D:\私人资料)
```

不会遍历内部文件。

优点：

- 速度最快
- 不影响文件结构
- 恢复简单
- 大目录性能稳定


---

# 3. 技术架构


```
                 FileHide.exe

                      |

                 Tauri Runtime

                      |

        ----------------------------

        Vue3 UI          Rust Backend

                            |

                       windows-rs

                            |

                    Windows API

                            |

                  SetFileAttributesW

                            |

                    NTFS 文件系统

```


---

# 4. 技术选型


## 前端

- Vue3
- TypeScript
- Vite
- TailwindCSS
- Element Plus


负责：

- UI展示
- 文件选择
- 状态管理
- 用户交互


---

## 后端

Rust：

- Tauri 2.x
- windows-rs
- serde
- rusqlite


负责：

- 文件属性操作
- Windows API调用
- 本地数据管理


---

# 5. 核心功能设计


# 5.1 隐藏文件


流程：

```
用户选择文件

↓

Vue调用Tauri Command

↓

Rust接收路径

↓

读取当前文件属性

↓

保存原属性

↓

设置隐藏属性

↓

保存数据库

↓

返回结果

```


---

# 5.2 隐藏文件夹


流程：

```
选择文件夹

↓

读取文件夹属性

↓

保存原始属性

↓

设置Hidden/System属性

↓

记录隐藏状态

```


注意：

禁止递归处理子文件。


---

# 5.3 恢复显示


流程：

```
用户点击恢复

↓

查询数据库

↓

获取原始属性

↓

恢复文件属性

↓

更新状态

```


要求：

恢复后保持隐藏前状态。


---

# 6. Windows API设计


## 隐藏


使用：

```
SetFileAttributesW
```


设置：

```
FILE_ATTRIBUTE_HIDDEN
FILE_ATTRIBUTE_SYSTEM
```


效果等同：

```
attrib +h +s 文件路径
```


---

## 恢复


恢复：

```
SetFileAttributesW
```


根据保存的原始属性恢复。


效果等同：

```
attrib -h -s 文件路径
```


---

# 7. 数据库设计


使用：

SQLite


数据库：

```
filehide.db
```


---

## hidden_item表


```sql
CREATE TABLE hidden_item
(
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    path TEXT NOT NULL,

    item_type TEXT NOT NULL,

    original_attributes INTEGER,

    current_status INTEGER,

    create_time DATETIME,

    update_time DATETIME
);
```


字段说明：


|字段|说明|
|-|-|
|id|主键|
|path|文件路径|
|item_type|FILE/FOLDER|
|original_attributes|隐藏前属性|
|current_status|状态|
|create_time|创建时间|
|update_time|更新时间|


---

# 8. UI设计


## 8.1 整体风格


Windows 11 Fluent Design。


特点：

- 深色主题
- 圆角卡片
- 简洁布局
- 紫蓝渐变主题


---

# 8.2 主窗口


尺寸：

```
1200 x 800
```


布局：

```
+------------------------------------------------+

| FileHide                                  - □ X |

+------------------------------------------------+

| Sidebar        |        Dashboard              |

|                |                                |

|                |                                |

+------------------------------------------------+

```


---

# 8.3 左侧菜单


宽度：

240px


菜单：


```
首页

隐藏文件

隐藏文件夹

历史记录

设置

关于

```


---

# 8.4 首页


展示：

- 隐藏项目数量
- 已保护数量
- 最近隐藏列表
- 当前保护状态


按钮：

```
+ 添加文件

+ 添加文件夹

```


---

# 9. Rust Command设计


## hide_file


输入：

```json
{
"path":"D:\\test.txt"
}
```


功能：

隐藏文件。


---

## hide_folder


输入：

```json
{
"path":"D:\\private"
}
```


功能：

隐藏文件夹。


---

## restore_item


输入：

```json
{
"id":1
}
```


功能：

恢复显示。


---

## list_items


功能：

查询隐藏记录。


---

# 10. 性能指标


测试目标：


|操作|目标|
|-|-|
|程序启动|<2秒|
|隐藏单文件|<100ms|
|隐藏文件夹|<100ms|
|恢复显示|<100ms|
|10000文件目录隐藏|<100ms|
|内存占用|<100MB|


---

# 11. 项目目录结构


```
filehide

├── src

│   ├── views

│   │   ├── Home.vue

│   │   ├── HiddenFiles.vue

│   │   └── Settings.vue

│   │

│   ├── components

│   └── api


├── src-tauri

│

│   ├── src

│   │

│   ├── main.rs

│   │

│   ├── commands

│   │   ├── hide.rs

│   │   ├── restore.rs

│   │   └── database.rs


└── package.json

```


---

# 12. 开发要求（提供给Codex）


请严格按照以下要求实现：

1. 使用 Rust + Tauri 2
2. 使用 Vue3 + TypeScript
3. UI按照设计稿实现
4. 文件操作全部由Rust完成
5. 不使用Node文件系统操作
6. 使用windows-rs调用Windows API
7. 隐藏文件夹禁止递归
8. 保存隐藏前文件属性
9. 支持恢复原始状态
10. SQLite保存历史记录
11. 最终生成Windows exe


---

# 13. 后续版本规划


## V2.0

增加：

- 系统托盘
- 快捷键隐藏
- 密码保护


## V3.0

增加：

- AES-256-GCM加密保险箱
- 虚拟磁盘
- 文件级安全保护


---

# 总结

FileHide V1.0 是一个高性能 Windows 隐藏工具。

核心实现：

```
选择文件/文件夹

↓

保存原属性

↓

SetFileAttributesW

↓

立即隐藏

```

特点：

- 秒级响应
- 不修改数据
- 不递归扫描
- 低资源占用
- 易扩展为加密保险箱
