# 性能监控 (PerfMonitor)

一款跨平台桌面性能监控工具，基于 **Tauri + React + Rust** 构建。以悬浮水球的形式实时展示 CPU 使用率，展开后可查看完整的系统性能数据、进程管理、历史趋势和性能报告。

## 功能特色

### 悬浮水球
- CPU 使用率以水波动画直观展示
- 颜色随负载变化：绿色（正常）→ 黄色（中等）→ 红色（高负载）
- 可拖拽定位，自动吸附屏幕边缘
- 点击展开完整监控面板

### 实时监控
- **CPU** — 使用率、核心数、频率
- **内存** — 使用量、可用量、使用率
- **Swap** — 交换分区使用情况
- **磁盘** — 各分区使用量和使用率
- **系统负载** — 1/5/15 分钟平均负载

### 进程管理
- 实时进程列表，按 CPU / 内存排序
- 支持终止进程
- 进程资源占用排行榜

### 历史趋势
- ECharts 图表展示 CPU、内存、Swap 历史曲线
- 支持缩放和时间范围选择
- 数据本地持久化存储（SQLite）

### 性能报告
- 对比两个时间窗口的性能数据
- 自动生成分析结论和改善建议
- CPU / 内存 / Swap 对比改善百分比
- 卡顿评分系统

### 标记功能
- 在时间线上标记重要事件
- 方便对比系统变更前后的性能差异

### 系统集成
- 系统托盘图标，快速访问
- 开机自启动配置
- 主题切换（跟随系统 / 浅色 / 深色）
- 数据导出 CSV

## 技术栈

| 层级 | 技术 |
|------|------|
| 前端 | React 18 + TypeScript + Vite |
| 图表 | ECharts 5 |
| 后端 | Rust + Tauri 1.5 |
| 系统信息 | sysinfo crate（跨平台） |
| 数据存储 | SQLite (sqlx) |
| 样式 | 原生 CSS（支持深色模式） |

## 快速开始

### 环境要求

- **Node.js** >= 16
- **Rust** >= 1.70
- **npm** 或 **pnpm**

### 安装与运行

```bash
# 克隆项目
git clone https://github.com/Li-yuanchang/perf-monitor.git
cd perf-monitor

# 安装前端依赖
npm install

# 开发模式运行
npm run tauri dev

# 构建发布版本
npm run tauri build
```

## 项目结构

```
perf-monitor/
├── src/                    # 前端源码
│   ├── App.tsx             # 主组件
│   ├── style.css           # 样式（含深色模式）
│   └── main.tsx            # 入口
├── src-tauri/              # Rust 后端
│   ├── src/
│   │   ├── main.rs         # Tauri 命令 & 应用入口
│   │   ├── collector.rs    # 系统信息采集（sysinfo）
│   │   ├── store.rs        # SQLite 数据存储
│   │   └── macos.rs        # macOS 平台特性
│   ├── icons/              # 应用图标
│   └── Cargo.toml          # Rust 依赖
├── package.json
└── vite.config.ts
```

## 平台支持

- **macOS** — 完整支持（系统托盘、开机自启动、透明窗口）
- **Windows** — 基础支持（需适配部分平台特性）
- **Linux** — 基础支持（需适配部分平台特性）

> 核心性能采集使用 `sysinfo` crate，天然跨平台。平台差异主要在窗口效果和系统集成层面。

## 许可证

MIT License
