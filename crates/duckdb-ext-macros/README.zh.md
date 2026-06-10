# DuckDB 扩展宏

[![Crates.io](https://img.shields.io/crates/v/duckdb-ext-macros.svg)](https://crates.io/crates/duckdb-ext-macros)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

一个兼容 Rust 2024 Edition 的过程宏 crate，用于创建 DuckDB 可加载扩展。

本 crate 位于 [`cargo-duckdb-ext-tools`](https://github.com/redraiment/cargo-duckdb-ext-tools) workspace 中，也是 `cargo duckdb-ext new` 生成项目默认使用的入口点宏。

## 概述

`duckdb-ext-macros` 提供 `#[duckdb_extension]` 属性宏。该宏会把 Rust 初始化函数转换为 DuckDB 加载扩展时需要的 C 兼容入口点，并处理 FFI 样板代码和错误传播。

它是 DuckDB 官方 `duckdb-loadable-macros` crate 的现代替代方案，支持 Rust 2024 Edition，并与当前 workspace 的依赖版本保持一致。

## 特性

- **Rust 2024 Edition 支持**：兼容现代 Rust edition
- **生成 C API 入口点**：生成 `{extension_name}_init_c_api`
- **灵活配置**：通过宏属性或环境变量配置扩展名和 API 版本
- **错误传播**：将初始化失败转换为 DuckDB 错误消息
- **Workspace 集成**：由 `cargo-duckdb-ext-tools` 的项目模板使用
- **现代宏开发栈**：使用当前版本的 `darling`、`syn`、`quote` 和 `proc-macro2`

## 安装

通过 `cargo duckdb-ext new` 创建的项目会自动包含本 crate。自定义项目可在 `Cargo.toml` 中添加本 crate 和 DuckDB 可加载扩展依赖：

```toml
[dependencies]
duckdb = { version = "1.10503.1", features = ["vtab-loadable"] } # 或你需要的版本
duckdb-ext-macros = "0.2.0"
libduckdb-sys = { version = "1.10503.1", features = ["loadable-extension"] }
```

## 快速开始

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

使用配套 CLI 构建并打包扩展：

```bash
cargo duckdb-ext build -- --release
```

## 使用方法

### 基本用法

使用默认值的最简单用法：

```rust
#[duckdb_extension]
fn extension_entrypoint(connection: duckdb::Connection) -> Result<(), String> {
    // 扩展逻辑
    Ok(())
}
```

这将：

- 使用 Cargo 包名作为扩展名，并将 `-` 转换为 `_`
- 使用 DuckDB C API 版本 `v1.2.0` 作为最低要求版本
- 生成名为 `{package_name}_init_c_api` 的 C 入口点函数

### 自定义扩展名

```rust
#[duckdb_extension(name = "custom_extension")]
fn init(connection: duckdb::Connection) -> Result<(), String> {
    Ok(())
}
```

### 自定义 API 版本

```rust
#[duckdb_extension(api_version = "v1.1.0")]
fn init(connection: duckdb::Connection) -> Result<(), String> {
    Ok(())
}
```

### 环境变量

也可以通过环境变量配置扩展，这在构建脚本或 CI/CD 中很有用：

```bash
# 设置扩展名
export DUCKDB_EXTENSION_NAME="my_extension"

# 设置最低 API 版本
export DUCKDB_MINIMUM_API_VERSION="v1.2.0"
```

当宏属性未提供时，环境变量将作为回退选项使用。

## 工作原理

`#[duckdb_extension]` 宏会将 Rust 函数转换为 DuckDB 兼容的 C 入口点：

1. 解析宏属性（`name`、`api_version`）和回退环境变量
2. 解析扩展名
3. 解析最低 DuckDB C API 版本
4. 生成签名为 `{extension_name}_init_c_api` 的 C 兼容函数
5. 通过 `libduckdb_sys` 初始化 DuckDB Rust extension API
6. 从 DuckDB 原始数据库指针打开 `duckdb::Connection`
7. 调用用户的初始化函数
8. 将 Rust 错误转换为 DuckDB 错误消息

### 生成代码形态

对于以下输入：

```rust
#[duckdb_extension(name = "my_ext")]
fn init(conn: duckdb::Connection) -> Result<(), String> {
    Ok(())
}
```

宏会生成具有如下公开 ABI 形态的入口点：

```rust
#[unsafe(no_mangle)]
pub extern "C" fn my_ext_init_c_api(
    info: libduckdb_sys::duckdb_extension_info,
    access: *const libduckdb_sys::duckdb_extension_access,
) -> bool {
    // FFI 和错误处理代码
}
```

原始 Rust 函数会保留，并由生成的入口点调用。

## 配置选项

### 宏属性

- `name`：用于生成 C 入口点的扩展名
- `api_version`：最低要求的 DuckDB C API 版本，默认值为 `v1.2.0`

### 环境变量

- `DUCKDB_EXTENSION_NAME`：当未提供 `name` 时作为扩展名回退值
- `DUCKDB_MINIMUM_API_VERSION`：当未提供 `api_version` 时作为最低 API 版本回退值

### 名称解析顺序

1. 宏中的 `name` 属性
2. `DUCKDB_EXTENSION_NAME` 环境变量
3. `CARGO_PKG_NAME` 环境变量

解析后的名称会将 `-` 转换为 `_`，以兼容 C 符号命名。

### API 版本解析顺序

1. 宏中的 `api_version` 属性
2. `DUCKDB_MINIMUM_API_VERSION` 环境变量
3. 默认值：`v1.2.0`

## 错误处理

宏会处理：

- **编译时错误**：无效的宏语法或不支持的 item 类型
- **运行时错误**：扩展初始化失败
- **FFI 错误**：DuckDB C API 初始化和连接建立失败

错误会以描述性消息传播到 DuckDB。如果错误字符串无法转换为 C 字符串，生成代码会使用回退错误消息。

## 与 `cargo-duckdb-ext-tools` 的关系

本 crate 只负责生成扩展入口点，不会附加 DuckDB 的 534 字节扩展元数据页脚。打包扩展请使用 workspace 中的 CLI：

```bash
cargo duckdb-ext build -- --release
```

或者：

```bash
cargo duckdb-ext package \
  -i target/release/libmy_extension.dylib \
  -o my_extension.duckdb_extension \
  -v v1.0.0 \
  -p osx_arm64 \
  -d v1.4.2
```

## 从 `duckdb-loadable-macros` 迁移

如果当前正在使用 `duckdb-loadable-macros`，迁移方式很直接：

1. 更新 `Cargo.toml`：

   ```toml
   # 删除或替换：
   # duckdb-loadable-macros = "..."

   # 添加：
   duckdb-ext-macros = "0.2.0"
   ```

2. 更新导入：

   ```rust
   // 从：
   // use duckdb_loadable_macros::duckdb_entrypoint;

   // 改为：
   use duckdb_ext_macros::duckdb_extension;
   ```

3. 更新属性：

   ```rust
   // 从：
   // #[duckdb_entrypoint]

   // 改为：
   #[duckdb_extension]
   ```

扩展注册逻辑通常无需修改。

## 许可证

本项目基于 MIT 许可证。详情见 workspace 根目录的 [`LICENSE`](../../LICENSE) 文件。

## 致谢

- DuckDB 团队提供了优秀的数据库和原始的 `duckdb-loadable-macros`
- Rust 社区提供了优秀的过程宏工具链
- 本项目的所有贡献者
