# DuckDB 扩展宏

一个现代化的、兼容 Rust 2024 Edition 的过程宏，用于创建 DuckDB 可加载扩展。

## 概述

本 crate 提供了 `#[duckdb_extension]` 属性宏，用于简化在 Rust 中创建 DuckDB 可加载扩展的过程。它作为 DuckDB 官方 `duckdb-loadable-macros` crate 的现代替代品，官方版本仅支持 Rust 2021 Edition。

## 特性

- **Rust 2024 Edition 支持**：完全兼容现代 Rust 版本
- **直接替换**：API 兼容 `duckdb-loadable-macros`
- **灵活配置**：通过属性或环境变量配置扩展名和 API 版本
- **全面的错误处理**：正确地将 Rust 错误传播到 DuckDB 的 C API
- **安全的 FFI 集成**：通过适当的错误处理管理不安全操作
- **现代依赖**：使用最新版本的 `darling`、`syn`、`quote` 和 `proc-macro2`

## 安装

在 `Cargo.toml` 中添加：

```toml
[dependencies]
duckdb = { version = "1.4.2", features = ["vtab-loadable"] }  # 或您喜欢的版本
duckdb-ext-macros = "0.1.0"
libduckdb-sys = { version = "1.4.2", features = ["loadable-extension"] }
```

## 快速开始

```rust
use duckdb_ext_macros::duckdb_extension;

#[duckdb_extension(name = "my_extension", api_version = "v1.2.0")]
fn my_extension_init(connection: duckdb::Connection) -> Result<(), String> {
    // 在这里编写您的扩展初始化逻辑
    // 注册函数、类型或执行设置

    println!("我的扩展加载成功！");
    Ok(())
}
```

## 使用方法

### 基本用法

使用默认值的最简单用法：

```rust
#[duckdb_extension]
fn my_extension(connection: duckdb::Connection) -> Result<(), String> {
    // 扩展逻辑
    Ok(())
}
```

这将：
- 使用您的 cargo 包名作为扩展名（转换为 snake_case）
- 使用 DuckDB API 版本 "v1.2.0" 作为最低要求版本
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

您也可以通过环境变量配置扩展，这在构建脚本或 CI/CD 中很有用：

```bash
# 设置扩展名
export DUCKDB_EXTENSION_NAME="my_extension"

# 设置最低 API 版本
export DUCKDB_MINIMUM_API_VERSION="v1.2.0"
```

当宏属性未提供时，环境变量将作为回退选项使用。

## 工作原理

`#[duckdb_extension]` 宏将您的 Rust 函数转换为 DuckDB 兼容的 C 入口点。以下是底层发生的过程：

1. **参数解析**：宏解析属性（`name`、`api_version`）或读取环境变量
2. **名称解析**：通过回退逻辑确定扩展名
3. **API 版本解析**：确定最低要求的 DuckDB API 版本
4. **代码生成**：生成签名如 `{extension_name}_init_c_api` 的 C 兼容函数
5. **FFI 集成**：创建围绕 DuckDB C API 调用的安全包装器
6. **错误处理**：将 Rust 错误转换为 DuckDB 错误消息

### 生成的代码示例

对于以下输入：
```rust
#[duckdb_extension(name = "my_ext")]
fn init(conn: duckdb::Connection) -> Result<(), String> { Ok(()) }
```

宏将生成：
```rust
#[unsafe(no_mangle)]
pub extern "C" fn my_ext_init_c_api(
    info: libduckdb_sys::duckdb_extension_info,
    access: *const libduckdb_sys::duckdb_extension_access
) -> bool {
    // FFI 和错误处理代码
}
```

## 与官方 `duckdb-loadable-macros` 的比较

| 特性 | `duckdb-ext-macros` | `duckdb-loadable-macros` |
|------|---------------------|--------------------------|
| Rust 版本 | **2024 Edition** | 仅支持 2021 Edition |

## 从 `duckdb-loadable-macros` 迁移

如果您当前正在使用 `duckdb-loadable-macros`，迁移非常简单：

1. **更新 Cargo.toml**：
   ```toml
   # 删除或替换：
   # duckdb-loadable-macros = "..."

   # 添加：
   duckdb-ext-macros = "0.1.0"
   ```

2. **更新导入**：
   ```rust
   // 从：
   // use duckdb_loadable_macros::duckdb_entrypoint;

   // 改为：
   use duckdb_ext_macros::duckdb_extension;
   ```

3. **更新属性**：
   ```rust
   // 从：
   // #[duckdb_entrypoint]

   // 改为：
   #[duckdb_extension]
   ```

API 设计为兼容，因此您现有的扩展逻辑应该无需更改即可工作。

## 配置选项

### 宏属性

- `name`：扩展名（用于 C 入口点函数）
- `api_version`：最低要求的 DuckDB C API 版本（默认："v1.2.0"）

### 环境变量

- `DUCKDB_EXTENSION_NAME`：扩展名（当 `name` 属性未提供时作为回退）
- `DUCKDB_MINIMUM_API_VERSION`：最低 API 版本（当 `api_version` 属性未提供时作为回退）

### 名称解析顺序

扩展名按以下顺序确定：
1. 宏中的 `name` 属性
2. `DUCKDB_EXTENSION_NAME` 环境变量
3. `CARGO_PKG_NAME` 环境变量（cargo 包名）

### API 版本解析顺序

最低 API 版本按以下顺序确定：
1. 宏中的 `api_version` 属性
2. `DUCKDB_MINIMUM_API_VERSION` 环境变量
3. 默认值："v1.2.0"

## 错误处理

宏提供全面的错误处理：

1. **编译时错误**：无效的宏语法或配置
2. **运行时错误**：扩展初始化失败
3. **FFI 错误**：C API 交互失败

错误会以描述性消息传播到 DuckDB。如果错误字符串分配失败，将使用回退消息。

## 许可证

本项目采用与 DuckDB 相同的许可证（MIT 许可证）。

## 致谢

- DuckDB 团队提供了优秀的数据库和原始的 `duckdb-loadable-macros`
- Rust 社区提供了优秀的过程宏库
- 本项目的所有贡献者