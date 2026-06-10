# cargo-duckdb-ext-tools

[![Crates.io](https://img.shields.io/crates/v/cargo-duckdb-ext-tools.svg)](https://crates.io/crates/cargo-duckdb-ext-tools)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.93%2B-blue.svg)](https://www.rust-lang.org)

一个面向 Rust 开发者的 DuckDB Extension Cargo workspace。仓库内同时维护 `cargo-duckdb-ext` CLI 和配套的 `duckdb-ext-macros` 过程宏，让扩展项目脚手架、入口点宏、构建和打包流程可以在同一个仓库里协同演进。

## 概述

DuckDB 扩展是动态库文件（`.dylib`/`.so`/`.dll`），在文件末尾附加了一个 534 字节的元数据页脚。官方的 DuckDB Rust 扩展模板依赖 Python 脚本来添加此元数据，要求开发者同时维护 Rust 和 Python 环境。

本 workspace 将 DuckDB 扩展开发流程尽量收敛在 Rust 生态内：

| Package | 路径 | 职责 |
|---------|------|------|
| `cargo-duckdb-ext-tools` | `.` | Cargo 插件，安装后命令为 `cargo-duckdb-ext`，提供 `new`、`build`、`package`、`meta` 子命令 |
| `duckdb-ext-macros` | `crates/duckdb-ext-macros` | 过程宏 crate，提供 `#[duckdb_extension]`，用于生成 DuckDB C API 入口点 |

### 主要特性

- **零非 Rust 依赖**：纯 Rust 实现，无需 Python 或额外运行时工具
- **完整的开发工作流**：项目脚手架、构建、打包、元信息查看，一体化工具
- **同仓维护入口点宏**：生成项目使用的 `duckdb-ext-macros` 与 CLI 模板在同一仓库维护
- **智能默认值**：从 Cargo 元数据和构建工件自动推断参数
- **跨平台支持**：支持主要平台的原生构建和交叉编译
- **专业日志输出**：兼容 Cargo 的输出格式，支持颜色
- **元信息查看与编辑**：无需外部脚本即可读取和更新 DuckDB 扩展元信息页脚
- **ABI 版本支持**：支持稳定（`C_STRUCT`）和不稳定（`C_STRUCT_UNSTABLE`）ABI 类型

## 安装

安装 CLI：

```bash
cargo install cargo-duckdb-ext-tools
```

工具安装后命令为 `cargo-duckdb-ext`，提供四个子命令：`new`、`build`、`package` 和 `meta`。

配套过程宏通常会在执行 `cargo duckdb-ext new` 时自动加入新项目。如果手写扩展项目、不使用生成器，请和 DuckDB 可加载扩展依赖一起添加：

```toml
[dependencies]
duckdb = { version = "1.10503.1", features = ["vtab-loadable"] }
duckdb-ext-macros = "0.2.0"
libduckdb-sys = { version = "1.10503.1", features = ["loadable-extension"] }
```

## 完整工作流程示例

以下是从项目创建到在 DuckDB 中运行扩展的完整示例：

```bash
# 1. 创建新的扩展项目
cargo duckdb-ext new quack

# 2. 进入项目目录
cd quack

# 3. 构建和打包扩展
cargo duckdb-ext build -- --release

# 4. 扩展现在已准备就绪：
# target/release/quack.duckdb_extension

# 5. 检查扩展元信息
cargo duckdb-ext meta target/release/quack.duckdb_extension

# 6. 在 DuckDB 中加载并测试扩展
duckdb -unsigned -c "load 'target/release/quack.duckdb_extension'; from quack('Joe')"
```

**预期输出：**

```text
┌───────────────┐
│     🐥       │
│   varchar     │
├───────────────┤
│ Hello Joe     │
└───────────────┘
```

这将创建一个表函数扩展，为任意输入名称返回 `Hello {name}`。

## 子命令

### 1. `cargo duckdb-ext new` - 创建新扩展项目

创建完整的 DuckDB 扩展项目，包含正确的配置和模板代码。

```bash
cargo duckdb-ext new [选项] <路径>
```

#### 基本用法

```bash
# 创建表函数扩展（默认）
cargo duckdb-ext new my-extension

# 创建标量函数扩展
cargo duckdb-ext new --scalar my-scalar-extension
```

#### 关键选项

- `--table` / `--scalar`：选择函数类型（表函数为默认）
- `--name <名称>`：设置包名（默认为目录名）
- `--edition <年份>`：Rust edition（2015、2018、2021、2024）
- `--vcs <版本控制系统>`：版本控制系统（git、hg、pijul、fossil、none）

#### 创建内容

- 包含 `cdylib` 配置的完整 Cargo 项目
- DuckDB 依赖（`duckdb`、`libduckdb-sys`、`duckdb-ext-macros`）
- 使用 `#[duckdb_extension]` 的 DuckDB 入口点模板代码
- 表函数或标量函数示例实现
- 发布配置优化（LTO、strip）

### 2. `cargo duckdb-ext build` - 构建和打包扩展

高级命令，结合编译和打包，具有智能默认值。

```bash
cargo duckdb-ext build [选项] [-- <CARGO_BUILD参数>...]
```

#### 基本用法

```bash
# 使用发布优化构建
cargo duckdb-ext build -- --release

# 从 macOS 交叉编译到 Linux
cargo duckdb-ext build -- --release --target x86_64-unknown-linux-gnu
```

#### 智能默认值

工具自动检测：

- **库路径**：从构建输出中的 `cdylib` 工件推断
- **扩展路径**：库旁边的 `<包名>.duckdb_extension`
- **扩展版本**：从 `Cargo.toml` 版本字段推断（前缀为 `v`）
- **平台**：从目标三元组或主机系统推断
- **DuckDB 版本**：从 `duckdb` 或 `libduckdb-sys` 依赖推断

#### 关键选项

- `-m, --manifest-path`：`Cargo.toml` 路径
- `-o, --extension-path`：覆盖输出扩展路径
- `-v, --extension-version`：覆盖扩展版本
- `-p, --duckdb-platform`：覆盖目标平台
- `-d, --duckdb-version`：指定 DuckDB 版本
- `-a, --duckdb-capi-version`：指定 DuckDB C API 版本（启用稳定 ABI）

### 3. `cargo duckdb-ext package` - 打包现有库

低级命令，将 DuckDB 元数据附加到现有动态库。

```bash
cargo duckdb-ext package --library-path <库路径> --extension-path <输出路径> \
  --extension-version <版本> --duckdb-platform <平台> \
  (--duckdb-version <版本> | --duckdb-capi-version <版本>)
```

#### 示例

```bash
cargo duckdb-ext package \
  -i target/release/libmy_extension.dylib \
  -o my_extension.duckdb_extension \
  -v v1.0.0 \
  -p osx_arm64 \
  -d v1.4.2
```

### 4. `cargo duckdb-ext meta` - 查看和更新扩展元信息

读取 `.duckdb_extension` 文件中 534 字节的 DuckDB 元信息页脚，或就地更新指定字段。如果目标文件尚不包含元信息页脚且提供了更新选项，该命令会自动追加一个新的页脚。

```bash
cargo duckdb-ext meta [选项] <扩展文件路径>
```

#### 基本用法

```bash
# 只读模式：展示元信息，不修改扩展文件
cargo duckdb-ext meta target/release/quack.duckdb_extension

# 编辑模式：更新指定字段
cargo duckdb-ext meta \
  -v v1.0.1 \
  -p osx_arm64 \
  -a v1.2.0 \
  target/release/quack.duckdb_extension
```

#### 关键选项

- `--field-1`、`--field-2`、`--field-3`：以小写十六进制形式更新保留的 32 字节字段
- `-v, --extension-version`：更新扩展版本
- `-d, --duckdb-version`：更新 DuckDB 版本并设置 ABI 类型为 `C_STRUCT_UNSTABLE`
- `-a, --duckdb-capi-version`：更新 DuckDB C API 版本并设置 ABI 类型为 `C_STRUCT`
- `-p, --duckdb-platform`：更新 DuckDB 平台标识符
- `-m, --magic-version`：更新元信息的魔术版本
- `-s, --signature`：以小写十六进制形式更新 256 字节的签名区域

不提供任何选项时，`meta` 为只读模式。十六进制字段必须使用小写字母且不能超过各自的固定长度。

## 配套过程宏 crate

`duckdb-ext-macros` 提供生成项目使用的 `#[duckdb_extension]` 属性宏：

```rust
use duckdb::Connection;
use duckdb_ext_macros::duckdb_extension;
use std::error::Error;

#[duckdb_extension(name = "my_extension", api_version = "v1.2.0")]
pub fn extension_entrypoint(connection: Connection) -> Result<(), Box<dyn Error>> {
    // 注册函数、类型或执行初始化逻辑。
    Ok(())
}
```

该宏会生成名为 `{extension_name}_init_c_api` 的 C 兼容函数，初始化 DuckDB Rust extension API，打开 `duckdb::Connection`，调用用户初始化函数，并将初始化错误传递给 DuckDB。宏的完整说明见 [`crates/duckdb-ext-macros/README.zh.md`](crates/duckdb-ext-macros/README.zh.md)。

## 平台支持

### 支持平台

- **macOS**：Apple Silicon (arm64) 和 Intel (x86_64)
- **Linux**：x86_64、aarch64、x86、arm
- **Windows**：x86_64、x86（通过交叉编译）

### 平台映射

工具自动将 Rust 目标三元组映射到 DuckDB 平台标识符：

| Rust 目标三元组 | DuckDB 平台 |
|----------------|-------------|
| `x86_64-apple-darwin` | `osx_amd64` |
| `aarch64-apple-darwin` | `osx_arm64` |
| `x86_64-unknown-linux-gnu` | `linux_amd64` |
| `aarch64-unknown-linux-gnu` | `linux_arm64` |
| `x86_64-pc-windows-msvc` | `windows_amd64` |
| `i686-pc-windows-msvc` | `windows_amd` |

## 技术细节

### ABI 类型

- **`C_STRUCT_UNSTABLE`**：针对 DuckDB 版本构建扩展时的默认值
- **`C_STRUCT`**：指定 `--duckdb-capi-version` 时使用（稳定 ABI）

### 元数据结构

534 字节的页脚包括：

1. 起始签名（22 字节）
2. 3 个保留字段（96 字节）
3. ABI 类型（32 字节）
4. 扩展版本（32 字节）
5. DuckDB/C API 版本（32 字节）
6. 平台标识符（32 字节）
7. 魔术版本 `4`（32 字节）
8. 签名区域（256 字节；全零表示未签名）

`package` 和 `build` 子命令在创建 `.duckdb_extension` 文件时会写入此页脚。`meta` 子命令可展示相同字段、更新指定值，或为尚无页脚的文件追加元信息。

## 项目结构

```text
.
├── Cargo.toml                  # Workspace manifest 和 CLI package manifest
├── src/                        # cargo-duckdb-ext-tools CLI/library 代码
│   ├── main.rs                 # CLI 入口点
│   ├── lib.rs                  # 库导出
│   ├── error.rs                # 错误类型
│   ├── logging.rs              # 兼容 Cargo 的日志
│   ├── helpers.rs              # 文件系统工具
│   ├── commands/               # 子命令实现
│   └── options/                # CLI 选项解析
└── crates/
    └── duckdb-ext-macros/      # #[duckdb_extension] 过程宏 crate
        ├── Cargo.toml
        └── src/lib.rs
```

## 测试

```bash
# 以开发模式构建默认 CLI package
cargo build

# 运行默认 CLI package 的测试
cargo test

# 检查配套过程宏 crate
cargo check -p duckdb-ext-macros

# 构建或测试整个 workspace
cargo build --workspace
cargo test --workspace

# 本地安装 CLI 进行测试
cargo install --path .
```

## 贡献

欢迎贡献，请随时提交 Pull Request。

1. Fork 本仓库
2. 创建您的功能分支（`git checkout -b feature/amazing-feature`）
3. 提交您的更改（`git commit -m 'Add some amazing feature'`）
4. 推送到分支（`git push origin feature/amazing-feature`）
5. 打开 Pull Request

## 支持

- **GitHub Issues**：[https://github.com/redraiment/cargo-duckdb-ext-tools/issues](https://github.com/redraiment/cargo-duckdb-ext-tools/issues)
- **邮箱**：Zhang, Zepeng <redraiment@gmail.com>

## 许可证

本项目基于 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。

