# OnetCli UI 设计系统重构设计

日期：2026-05-12

## 背景

OnetCli 是基于 GPUI 的跨平台桌面客户端，覆盖数据库、Redis、MongoDB、SSH/SFTP、终端与 AI 工作流。当前 UI 已具备完整功能，但不同页面在视觉密度、工具栏结构、侧栏层级、面板边界、状态反馈和组件细节上存在不一致，导致长期使用时扫描效率和专业质感不足。

本次重构目标不是换皮，而是建立一套面向高频桌面工具的统一设计系统，并用首页作为第一块真实落地样板，再逐步扩展到各工作台。

## 已确认决策

- 重构范围优先级：全应用统一设计系统优先。
- 视觉方向：精密原生工具，保留桌面工具气质，提升密度、对齐、边框、状态色和组件一致性。
- 共享层边界：允许修改 `crates/ui` 中的共享 `gpui-component` 组件和主题系统。
- 主题范围：第一轮重点保证 `Default Light` 与 `macOS Classic Dark`，其它主题保持可用但不做精调。
- 信息架构：允许较大调整，包括首页、工作台面板、AI 助手入口和设置结构。
- 执行主线：先统一设计系统，再用首页完整验证，之后迁移工作台。

## 目标

1. 建立统一的主题 token 和组件状态规范，让主应用、故事集和业务工作台共享同一套视觉语言。
2. 将首页升级为连接与工作区的 Command Home，承接全局搜索、新建、同步、密钥、登录和最近使用等入口。
3. 将数据库、Redis、MongoDB、终端、SFTP 和 ChatDB 逐步对齐到统一工作台结构。
4. 保持现有连接协议、数据库操作、终端渲染、SFTP 文件行为、认证和云同步业务逻辑不变。

## 非目标

- 不重写数据库表格编辑、查询执行、终端渲染、SFTP 文件传输、云同步、认证、License 或连接协议逻辑。
- 不在第一轮精调所有第三方主题文件。
- 不引入全新的 UI 框架或跨技术栈 Web 前端。
- 不为了视觉重构改变已存在的数据存储格式。

## 设计系统

### 主题 Token

第一轮聚焦 `crates/ui/src/theme/default-theme.json` 中的 `Default Light` 与 `macOS Classic Dark`。需要统一以下类别：

- 基础层：`background`、`foreground`、`border`、`muted`、`muted.foreground`、`secondary`。
- 操作层：`primary`、`primary.hover`、`primary.active`、`danger`、`warning`、`success`。
- 导航层：`sidebar`、`sidebar.accent`、`tab`、`tab.active`、`tab_bar`、`title_bar`。
- 数据层：`table`、`table.head`、`table.hover`、`table.active`、`list`、`list.hover`、`list.active`。
- 浮层层：`popover`、`overlay`、`ring`、`input.border`。

原则是降低随机色值和局部硬编码，让页面尽量通过主题 token 表达状态。必要时可以补充少量 token，但应优先复用已有字段，避免 schema 和主题维护成本扩大。

### 共享组件

优先统一以下组件的尺寸、圆角、边框、焦点、hover、active、disabled 和文本层级：

- `Button`、`ButtonIcon`、`DropdownButton`、`Toggle`
- `Input`、`Search`、`Select`、`NumberInput`
- `Sidebar`、`SidebarMenuItem`、`List`、`ListItem`、`Tree`
- `Tab`、`TabBar`、工作台工具栏样式
- `Popover`、`Dialog`、`Sheet`
- `Table`、`one_ui::EditTable`

组件默认风格应偏桌面原生工具：边界清晰、可扫描、不过度圆角、不过度阴影。卡片圆角以 8px 或更小为主；工具栏和列表项保持稳定高度，避免 hover 或选中态造成布局跳动。

## 信息架构

### 首页：Command Home

首页从单纯连接列表升级为全局连接管理入口：

- 左侧增加全局模块 rail，用于区分首页、连接、AI、设置等高层入口。
- 第二层为连接类型与工作区过滤，保留数据库、SSH/SFTP、Redis、MongoDB、ChatDB、串口等分类。
- 主内容区支持工作区分组、连接卡片、后续列表视图、最近使用和空状态。
- 顶部工具栏集中放置新建连接、同步、密钥状态、全局搜索、刷新和工作区筛选。

首页必须验证登录态、未登录态、同步中、同步冲突、主密钥锁定/解锁、无连接、无搜索结果和有大量连接时的视觉表现。

### 工作台：统一三栏

各业务工作台统一为以下空间模型：

- 左侧资源面板：数据库对象树、Redis/MongoDB 树、SFTP 文件树、终端会话或历史。
- 中央主工作区：表格、SQL 编辑器、终端、文件列表、结果视图或图表。
- 右侧 AI/Context 面板：基于当前连接、对象、选中数据、终端路径或文件上下文提供 AI 辅助。

右侧 AI 面板必须可收起，不能阻塞主工作区。数据库、终端、SFTP、ChatDB 页面使用统一的分隔线、工具栏高度、搜索位置、面板收起行为和状态反馈。

## 页面落地范围

### 第一阶段：共享组件与主题

修改重点在 `crates/ui`、`crates/one_ui` 与 `crates/story`：

- 统一主主题 token。
- 在 Story 中覆盖按钮、输入、列表、侧栏、tab、表格、popover、dialog 的主状态。
- 降低局部硬编码色值，优先改为 `cx.theme()` token。
- 建立工具栏、侧栏和数据面板的复用样式约定。

### 第二阶段：首页样板

修改重点在 `main/src/home_tab.rs`、`main/src/home/*`、`main/src/user_avatar.rs`：

- 重排首页导航和连接类型过滤。
- 重构顶部工具栏的信息层级。
- 统一连接卡片的尺寸、图标区、名称、描述、团队标记、活动状态和 hover 操作。
- 补齐空状态、搜索无结果、工作区为空、密钥锁定、同步冲突等状态样式。
- 保留双击打开、单击选中、拖拽排序、编辑、复制、删除、打开 SFTP 等现有行为。

### 第三阶段：工作台对齐

修改重点逐步覆盖以下模块：

- `crates/db_view`：数据库树、对象列表、SQL 编辑器、结果区、AI SQL 面板。
- `crates/redis_view` 与 `crates/mongodb_view`：资源树、键/集合视图、详情面板。
- `crates/terminal_view` 与 `crates/sftp_view`：终端侧栏、SFTP 双栏、路径栏、文件列表。
- `main/src/setting_tab.rs` 与 `main/src/new_connection/*`：设置页和新建连接窗口。

该阶段以结构和视觉对齐为主，不改变业务命令或数据协议。

## 数据与事件流

主题 token 从 `gpui_component::Theme` 读取，通过 `ActiveTheme` 暴露给各组件。共享组件只消费主题和尺寸，不持有业务状态。

首页仍由 `HomePage` 管理连接、工作区、搜索、筛选、同步、登录和密钥状态。UI 重构不得改变以下事件语义：

- 连接创建、更新、删除后刷新列表并触发必要同步。
- 双击连接按原策略打开对应 tab。
- 拖拽排序只在同一工作区和允许排序时生效。
- 登录、同步、冲突解决和密钥解锁沿用现有服务。

工作台页面沿用当前实体、事件和 tab 注册机制。右侧 AI/Context 面板只调整入口和布局边界，不改变各模块现有 AI 请求语义。

## 错误处理与状态

UI 状态需要显式覆盖：

- 组件 disabled、loading、focus、selected、hover、active。
- 连接列表 loading、empty、filtered empty、syncing、conflict、locked、offline。
- 表格和对象树 loading、empty、error、selected。
- AI 面板 collapsed、loading、streaming、error、no context。

错误状态必须保持可读，不用颜色作为唯一提示。暗色模式下 danger、warning、success 必须具备足够对比度。

## 测试与验证

基础命令：

```bash
cargo fmt --check
cargo check -p gpui-component
cargo check -p story
cargo check -p main
```

视觉冒烟：

```bash
cargo run -p story
cargo run -p main
```

人工检查清单：

- `Default Light` 与 `macOS Classic Dark` 下组件状态可读。
- 首页在有连接、无连接、无搜索结果、多工作区、同步中、冲突、密钥锁定、未登录状态下可用。
- 新建连接、编辑连接、复制连接、删除连接、打开连接、打开 SFTP、拖拽排序行为保持一致。
- 数据库对象树、表格、SQL 编辑器、AI 面板没有明显错位或文本重叠。
- SFTP 双栏、终端、设置页、新建连接窗口遵循统一间距和边界。

## 风险控制

- 共享组件改动影响面大：先在 `crates/story` 中验证组件状态，再进入主应用页面。
- 主题 token 变更可能影响所有主题：第一轮只精调两个主主题，其它主题只保证解析和基础可用。
- 信息架构调整可能影响用户路径：首页先作为样板落地，工作台迁移分阶段进行。
- 大文件风险：`main/src/home_tab.rs` 已经较大，实施时应优先提取局部渲染组件或 helper，避免继续扩大单文件复杂度。
- 视觉验证自动化不足：必须结合 `cargo check` 和人工视觉冒烟，不以编译通过替代 UI 验收。

## 验收标准

- Story 中核心组件状态与两个主主题表现一致。
- 首页完成新设计系统样板落地，并保留所有现有连接管理行为。
- 主工作台页面至少完成结构规则对齐，不出现明显视觉断裂。
- `cargo fmt --check`、`cargo check -p gpui-component`、`cargo check -p story`、`cargo check -p main` 可执行并报告结果。
- 若视觉冒烟无法运行，需要明确说明阻塞原因和未覆盖风险。
