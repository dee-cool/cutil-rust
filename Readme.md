# cutil-rust

Rust 通用工具库，提供数据库操作、消息队列、日志、工具函数等常用功能。

## 功能特性

### 数据库操作 (db_client, db_crud)

- **数据库连接管理**: 自动重连、健康检查、连接池管理
- **CRUD 操作宏**: 通过宏自动生成标准 CRUD 方法
- **自动更新功能**: 支持字段级别的智能更新
- **批量操作**: 支持批量创建、删除等操作
- **软删除**: 支持软删除和恢复功能
- **审计字段**: 自动管理创建时间、更新时间等审计字段
- **分页查询**: 内置分页功能

### 消息队列 (message_broker, message_center)

- **MQTT 支持**: 基于 rumqttc 的 MQTT 客户端实现
- **自动重连**: 支持指数退避重连策略
- **消息发布/订阅**: 完整的消息发布和订阅功能
- **HTTP 消息中心**: 支持通过 HTTP 发布消息

### 日志系统 (log)

- **结构化日志**: 基于 tracing 的结构化日志
- **文件输出**: 支持按日期滚动的日志文件
- **控制台输出**: 支持彩色控制台输出
- **日志级别**: 可配置的日志级别过滤

### 工具函数

- **字符串处理** (string, mask): 字符串工具和脱敏功能
- **时间处理** (time): 时间戳和格式化工具
- **环境变量** (env): 不区分大小写的环境变量读取
- **数据提取** (extract): 从 HashMap 中提取各种类型的数据
- **ID 生成** (generator): UUID、ULID、雪花 ID 生成
- **HTTP 客户端** (reqwest): 支持代理的 HTTP 客户端创建
- **分页** (pos, paged): 分页查询支持
- **懒加载** (lazyload): 异步懒加载器

### 错误处理 (meta, meta_config)

- **统一错误类型**: 自定义 `Meta` 错误类型
- **错误转换**: 自动转换各种错误类型到 `Meta`
- **HTTP 响应**: 支持将错误转换为 HTTP 响应

## 快速开始

### 添加依赖

```toml
[dependencies]
cutil-rust = "1.0.25092002"
```

### 数据库操作示例

```rust
use cutil_rust::cutil::db_client::DbClient;
use cutil_rust::cutil::db_crud::*;
use sea_orm::{ConnectOptions, Database};

// 创建数据库客户端
let options = ConnectOptions::new("postgres://user:pass@localhost/db".to_string());
let client = Arc::new(DbClient::with_options(options));

// 定义 Repository
impl_repository_struct!(UserRepo);
impl_standard_crud!(UserRepo, User, user, Uuid);

// 使用
let repo = UserRepo::new(client);
let user = repo.get(user_id).await?;
```

### 消息队列示例

```rust
use cutil_rust::cutil::message_broker::{MessageBrokerImpl, MessageBrokerOptions, Qos};

let options = MessageBrokerOptions {
    host: "localhost".to_string(),
    port: 1883,
    username: "user".to_string(),
    password: "pass".to_string(),
    ..Default::default()
};

let broker = MessageBrokerImpl::new(options)?;
broker.subscribe(vec!["topic/1".to_string()], Qos::AtLeastOnce).await?;
```

### 日志配置示例

```rust
use cutil_rust::cutil::log;
use tracing_subscriber::filter::LevelFilter;

log::configure(
    LevelFilter::INFO,
    "/var/log".to_string(),
    "myapp".to_string()
);
```

## 主要模块

- `db_client`: 数据库连接管理
- `db_crud`: CRUD 操作宏和工具
- `message_broker`: MQTT 消息代理
- `message_center`: 消息中心（MQTT + HTTP）
- `log`: 日志配置
- `meta`: 错误类型定义
- `meta_config`: 错误类型转换
- `time`: 时间工具
- `string`: 字符串工具
- `mask`: 数据脱敏
- `env`: 环境变量工具
- `extract`: 数据提取工具
- `generator`: ID 生成器
- `reqwest`: HTTP 客户端工具
- `pos`: 分页参数
- `paged`: 分页结果
- `lazyload`: 懒加载器
- `optionable`: Option 工具
- `resultable`: Result 工具
- `empty`: 空类型

## 设计模式

- **Repository 模式**: 通过宏实现标准化的数据访问层
- **依赖注入**: 使用 `more-di` 进行依赖管理
- **Trait 扩展**: 通过 trait 提供可组合的功能扩展
- **宏编程**: 使用声明式宏减少重复代码

## 依赖

主要依赖包括：

- `sea-orm`: 数据库 ORM
- `tokio`: 异步运行时
- `tracing`: 结构化日志
- `rumqttc`: MQTT 客户端
- `reqwest`: HTTP 客户端
- `serde`: 序列化/反序列化
- `more-di`: 依赖注入

## 许可证

MIT License

