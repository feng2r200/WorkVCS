# WorkVCS

[English](README.md) | **简体中文**

**面向 Agent 工作与知识状态的版本控制。**

[![CI](https://github.com/feng2r200/WorkVCS/actions/workflows/ci.yml/badge.svg)](https://github.com/feng2r200/WorkVCS/actions/workflows/ci.yml)

WorkVCS 为编码 Agent 提供可持久、可检查的状态，使工作可以跨越一次对话、一个进程或某个特定的 Agent 产品继续存在。它不仅对工作内容进行版本控制，也记录为什么当前状态值得相信：目标、计划、任务、决策、尝试、证据、验证、可复用知识，以及它们之间有类型的关系。

Git 对文件进行版本控制，任务管理工具描述工作分配，对话记录保存交流过程。WorkVCS 补上了三者之间缺失的一层：Agent 为了继续、解释、分支、合并、审查和恢复工作而需要持续演进的 **工作与知识状态（Work & Knowledge State）**。

> **项目状态：** 本仓库包含一个在限定范围内已达到本地发布就绪状态的 V1 实现。Rust 包版本仍为 `0.1.0`，尚未进行公开发布。源码采用 Apache-2.0 许可证；选择发布版本、发布构建产物和作出发布承诺仍是彼此独立的决策。

## 它有什么不同

- **语义化历史，而不是回放对话。** WorkVCS 将目标（Goal）、计划（Plan）、任务（Task）、决策（Decision）、发现（Finding）、风险（Risk）、知识（Knowledge）、验收标准（Acceptance Criterion）和验证（Verification）存储为显式且有版本的领域对象。
- **真正的分支与合并。** 工作分支可以分叉、比较和合并，冲突需要显式解决；系统可以恢复早期状态，并保留独立于 Git 分支的因果脉络。
- **确定性的恢复。** `context`、`resume`、`next` 和 `why` 根据结构、状态、证据与来源信息重建有界上下文，而不是从冗长对话中猜测。
- **协调而不负责编排。** Session、focus、claim、handoff 和合并进行中状态帮助多个 Agent 协作，同时 WorkVCS 保持独立于任何 Agent 运行时。
- **携带证据的状态。** 不可变事件、变更集、证据、资源观测和验证记录让完成声明可以被审计。
- **按逻辑项目路由。** ProjectRef 将语义项目归属与仓库、目录位置分离，因此不会仅仅因为当前 CWD 尚未绑定就遗漏本应持久化的内容。
- **本地优先且可移植。** 基于 SQLite 的存储、按 BLAKE3 寻址的对象、检查点和 bundle，让系统保持可检查、可迁移。

## 状态模型

WorkVCS 有意区分三类状态：

| 层 | 包含内容 | 是否进入工作状态版本历史？ |
| --- | --- | --- |
| 工作状态（Work State） | 工作、认知、知识、验收标准和关系 | 是 |
| 运行时协调（Runtime Coordination） | Session、focus、claim、handoff 和活动中的合并 | 否；以原子方式更新 |
| 来源记录（Provenance） | 变更集、事件、证据和已观测的资源基础 | 不可变审计轨迹 |

这种分层可以避免两种常见失败：把临时的 Agent 协调状态当成项目事实，以及把对话记录当成数据库。

## 项目感知的内容沉淀

ProjectRef v2 按确定顺序判断内容归属：显式指定的 ProjectRef、经过验证的语义项目或容器、Git 仓库，最后才是 CWD。只要存在更强且合格的归属，即使它尚未绑定，也会阻止向较弱上下文回退，避免仓库或目录在无提示的情况下接收本属于另一个逻辑项目的内容。

当集成层确认某项内容值得保留后，v2 `capture` 会先把不携带目标地址的意图写入中央日志。交付仍是一个显式且受摘要锁定的恢复操作：主项目拥有唯一的规范 Record，相关项目可以通过 CaptureGroup 获得不可变、固定到具体版本的引用。只读操作和明确的“不记录”决定保持零写入；只有针对注册表精确快照显式启用后，路由和准入才会生效。

如果迁移暴露出两个不同 ProjectRef 意外共享同一个未分区目标，WorkVCS 通过先预览、再以摘要锁定的隔离操作，只把被选中的绑定切换到确定性的全新 Store。旧 Store 与 ProjectRef 会被保留，不复制归属不明的历史状态；注册表版本改变后，两个激活标记必须分别显式刷新。

`workvcs project health` 可通过一次严格只读检查汇总注册表、完整绑定校验、激活标记、日志能力和可选 CWD 解析状态。持久操作恢复的规范命令是 `project operation-recovery`；`project capture-recovery` 继续作为同一 CaptureId 与日志状态机的可见兼容别名。

ADR-0519 已在源码树实现并完成精确的本地安装采用：在调用方显式授权时，单项目 `capture` 可以继续交付到准入时解析出的同一个、完整有效且非共享的既有绑定，并增加一个严格只读的开放操作清单。默认行为仍只写日志；ProjectRef 启动、目标变化、CaptureGroup 和历史积压处理仍分别需要授权。一个有界的同绑定现场 canary 及其零写入重放已经通过；随后单独授权的 Stage D 又只处理了当前 ProjectRef 下明确选中的两条历史回执，并保留其余全局清单的独立授权边界。

进一步说明请参阅 [ADR-0513（英文）](docs/decisions/adr/0513-projectref-durable-capture-routing.md)、[ADR-0517（英文）](docs/decisions/adr/0517-shared-project-binding-isolation.md)、[ADR-0518（英文）](docs/decisions/adr/0518-operator-control-plane-health-and-recovery-contract.md)、[ADR-0519（英文）](docs/decisions/adr/0519-authorized-existing-binding-delivery-and-operation-inventory.md)、[ProjectRef 控制平面契约（英文）](docs/architecture/projectref-control-plane-v2.md)和[迁移与验收契约（英文）](docs/architecture/projectref-registry-v2-migration-and-acceptance.md)。

## Agent 参与策略

随附的 Skill 默认按价值触发，因此仅安装 WorkVCS 不会强制每一个 Agent 任务采用同一种工作流。用户、项目或运行环境也可以声明“必须参与”策略。在该模式下，Agent 会在任务开始时选择 WorkVCS，在可能存在既有状态时进行有界读取，并在交接或结束前同步最小且真实的语义增量。项目文档可以继续作为说明性内容的权威来源，同时由 WorkVCS 作为必须使用的持久工作状态提供者；只要两者职责明确，双写是有效的。

“必须参与”只改变选择策略：它不会强制建立 Plan，也不会绕过迁移、激活、恢复交付、回滚或其他变更操作各自独立的授权与摘要锁。

在既有授权范围内，启动持久化操作的任务也必须负责普通的同目标完成与精确回读，不应让用户持续盯着常规待处理回执。历史开放条目只是分类证据，不是批处理队列；恢复前，Agent 必须把每条旧意图与当前项目事实对照，并说明“现在执行会改变什么”，避免把已完成、确定性终止、已被取代和目标已变化的操作当成同一种任务重放。

## 快速开始

前置要求：Git 和 Rust `1.98.1` 或更高版本。

```bash
git clone https://github.com/feng2r200/WorkVCS.git
cd WorkVCS
cargo build --release --locked -p workvcs-cli
./target/release/workvcs --version
./target/release/workvcs --help
```

创建本地配置：

```bash
mkdir -p "$HOME/.config/workvcs"
cp config.toml.example "$HOME/.config/workvcs/config.toml"
```

绑定一个项目并恢复它当前的工作上下文：

```bash
./target/release/workvcs config show
./target/release/workvcs project ensure --cwd /path/to/project
./target/release/workvcs resume --cwd /path/to/project
```

默认情况下，打包脚本只会构建带校验和的本地产物，不会执行安装：

```bash
scripts/package-workvcs.sh
```

安装必须显式执行：

```bash
scripts/package-workvcs.sh --install --bin-dir "$HOME/.local/bin"
```

默认情况下，`--install` 还会把随附的 Agent Skill 安装到 `$HOME/.agents/skills/workvcs`。如果只想安装二进制文件，请传入 `--no-install-skill`。

在将 WorkVCS 用作项目的持久化基础设施之前，请阅读[操作者快速开始与恢复指南（英文）](docs/operator/quickstart-and-recovery.md)。

## 设计导览

- [产品定义（英文）](docs/product/product-definition.md)——问题、产品承诺、用户模型和边界。
- [系统边界（英文）](docs/architecture/system-boundaries.md)——WorkVCS 与 Agent、Git、资源和知识空间之间的关系。
- [版本控制引擎（英文）](docs/architecture/versioning-engine.md)——工作状态的提交、分支、diff、合并、恢复和脉络。
- [语义操作与状态机（英文）](docs/architecture/semantic-operations-and-state-machines.md)——面向 Agent 的行为契约。
- [持久化模型（英文）](docs/architecture/persistence-model.md)和 [Schema 契约（英文）](docs/architecture/physical-schema-v0.1.md)——本地持久存储与可重建投影。
- [ProjectRef 归属与持久化捕获路由（英文）](docs/decisions/adr/0513-projectref-durable-capture-routing.md)——逻辑项目解析、日志准入、有界恢复和跨项目关联。
- [共享 ProjectRef 绑定隔离（英文）](docs/decisions/adr/0517-shared-project-binding-isolation.md)——零写入预览、精确替换、旧 Store 保留和激活刷新边界。
- [操作者控制平面健康与恢复契约（英文）](docs/decisions/adr/0518-operator-control-plane-health-and-recovery-contract.md)——单次完整只读健康检查、稳定错误分类和恢复命令兼容边界。
- [既有绑定授权交付与操作清单（英文）](docs/decisions/adr/0519-authorized-existing-binding-delivery-and-operation-inventory.md)——已完成本地安装采用的快速路径、积压可见性和保留的授权门。
- [V1 就绪台账（英文）](docs/provenance/v1-readiness-ledger.md)和[发布门矩阵（英文）](docs/provenance/v1-release-gate-matrix.md)——支撑限定成熟度声明的证据。
- [文档地图（英文）](docs/README.md)——完整的权威资料与证据索引。

## 范围边界

WorkVCS 有意不做以下事情：

- 替代 Git 或源代码远端仓库；
- 启动、规划或编排 Agent；
- 记录思维链或解析对话；
- 通过大语言模型进行语义推断；
- 提供云同步或托管式协作服务。

这些边界是架构的一部分，而不是尚未补齐的营销功能。它们让引擎保持可移植、可测试，并独立于任何一种 Agent。

## 开发

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --locked
scripts/validate-schema-v0.1.sh
scripts/smoke-v0.1-cli-workflow.sh
scripts/validate-projectref-acceptance-matrix.sh
```

提交改动前请阅读 [CONTRIBUTING.md（英文）](CONTRIBUTING.md)。安全问题请按照 [SECURITY.md（英文）](SECURITY.md)报告；一般帮助请查看 [SUPPORT.md（英文）](SUPPORT.md)。

## 许可证

WorkVCS 采用 [Apache License 2.0](LICENSE) 许可证。

Copyright © 2026 feng2r200.
