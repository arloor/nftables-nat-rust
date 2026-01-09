# WebUI 实现总结

本文档记录了为 nftables-nat-rust 项目添加 WebUI 的完整实现过程。

## 📋 实现内容

### 1. 项目结构重组

将原项目改造为 Cargo workspace 结构，支持多个 binary：

```
nftables-nat-rust/
├── Cargo.toml              # Workspace 配置
├── nat-cli/                # 原 NAT CLI 工具
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── config.rs
│       ├── ip.rs
│       ├── logger.rs
│       └── prepare.rs
└── webui/                  # 新增 WebUI
    ├── Cargo.toml
    ├── README.md
    ├── src/
    │   ├── main.rs         # 入口
    │   ├── server.rs       # 服务器配置
    │   ├── handlers.rs     # 请求处理
    │   ├── auth.rs         # JWT 认证
    │   └── config.rs       # 配置管理
    └── static/
        ├── login.html      # 登录页面
        └── index.html      # 主界面
```

### 2. 核心功能实现

#### 认证系统

- ✅ JWT 令牌认证
- ✅ 基于 bcrypt 的密码哈希
- ✅ HTTP-only Cookie 存储
- ✅ 7 天令牌过期时间
- ✅ 登录/登出功能

#### 配置管理

- ✅ 支持 TOML 配置文件读写
- ✅ 支持传统配置文件读写
- ✅ 配置文件热加载
- ✅ JSON 格式 API 接口

#### 规则查看

- ✅ 实时读取 nftables 规则
- ✅ 展示 IPv4 和 IPv6 规则
- ✅ HTML 和 JSON 双格式输出

#### TLS/HTTPS 支持

- ✅ 可选的 TLS 加密传输
- ✅ 支持自定义证书
- ✅ HTTP 模式（开发环境）
- ✅ HTTPS 模式（生产环境）

### 3. Web 界面

#### 登录页面 (login.html)

- 现代化设计，渐变背景
- 表单验证
- 错误提示
- 自动跳转已登录用户

#### 主界面 (index.html)

- 双标签页设计（配置管理 + 规则查看）
- 配置编辑器（支持 JSON 编辑）
- 实时规则查看
- 响应式布局
- 用户信息显示

### 4. API 端点

| 端点          | 方法 | 认证 | 功能             |
| ------------- | ---- | ---- | ---------------- |
| `/api/login`  | POST | 否   | 用户登录         |
| `/api/logout` | POST | 否   | 用户登出         |
| `/api/me`     | GET  | 是   | 获取当前用户     |
| `/api/config` | GET  | 是   | 获取配置         |
| `/api/config` | POST | 是   | 保存配置         |
| `/api/rules`  | GET  | 是   | 获取规则（JSON） |
| `/rules`      | GET  | 是   | 获取规则（HTML） |
| `/health`     | GET  | 否   | 健康检查         |

### 5. 技术栈

#### 后端

- **axum 0.8** - Web 框架
- **axum-bootstrap 0.1** - 服务器启动工具
- **tower 0.5** - 中间件
- **tower-http 0.6** - HTTP 中间件
- **jsonwebtoken 10** - JWT 认证
- **bcrypt 0.17** - 密码哈希
- **tokio** - 异步运行时
- **serde** - 序列化/反序列化
- **toml** - TOML 解析

#### 前端

- 纯 HTML + CSS + JavaScript
- 无需构建工具
- 现代化 UI 设计
- Fetch API 异步请求

## 🔒 安全特性

1. **认证机制**

   - JWT 令牌认证
   - HTTP-only Cookie（防止 XSS）
   - 密码 bcrypt 哈希存储
   - 可配置 JWT 密钥

2. **传输安全**

   - 支持 TLS 1.2/1.3
   - HTTPS 加密传输
   - 可使用自签名或正式证书

3. **访问控制**
   - 所有管理接口需要认证
   - 令牌自动过期
   - 支持主动登出

## 📦 编译产物

- **nat**: 3.5MB (原 NAT CLI 工具)
- **webui**: 14MB (Web 管理界面)

## 🚀 使用方式

### 快速启动

```bash
# 编译
cargo build --release --package webui

# 启动（HTTP）
./target/release/webui -u admin --password mypass --toml-config nat.toml

# 启动（HTTPS）
./target/release/webui -u admin --password mypass \
  --toml-config nat.toml \
  --cert cert.pem --key key.pem
```

### systemd 服务

参考 `webui/README.md` 和 `WEBUI_USAGE.md` 中的 systemd 配置。

## 📝 文档

- **webui/README.md** - WebUI 使用说明
- **WEBUI_USAGE.md** - 详细使用教程
- **README.md** - 主项目说明（已更新）

## ⚙️ 配置选项

| 参数                  | 说明      | 必需       | 默认值   |
| --------------------- | --------- | ---------- | -------- |
| `-p, --port`          | 监听端口  | 否         | 8080     |
| `-u, --username`      | 用户名    | 是         | -        |
| `--password`          | 密码      | 是         | -        |
| `--jwt-secret`        | JWT 密钥  | 否         | 默认密钥 |
| `--cert`              | TLS 证书  | HTTPS 需要 | -        |
| `--key`               | TLS 私钥  | HTTPS 需要 | -        |
| `--toml-config`       | TOML 配置 | 二选一     | -        |
| `--compatible-config` | 传统配置  | 二选一     | -        |

## 🎯 设计决策

### 为什么使用 Workspace?

- 代码分离：CLI 和 WebUI 独立开发
- 共享依赖：减少编译时间
- 独立发布：可单独发布不同组件

### 为什么选择 axum?

- 性能优秀，基于 tokio
- 类型安全的路由和提取器
- 良好的生态系统
- 活跃的社区支持

### 为什么支持两种配置格式?

- 向后兼容：保持对传统格式的支持
- 灵活性：用户可选择适合的格式
- 渐进迁移：允许用户逐步迁移

## 🔧 开发建议

### 本地开发

```bash
# 启用日志
RUST_LOG=debug cargo run --package webui -- \
  -u admin --password admin --toml-config nat.toml
```

### 测试

```bash
# 测试登录
curl -X POST http://localhost:8080/api/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"admin"}' \
  -c cookies.txt

# 测试获取配置
curl http://localhost:8080/api/config -b cookies.txt

# 测试获取规则
curl http://localhost:8080/api/rules -b cookies.txt
```

## 📈 后续改进建议

1. **CSRF 保护**

   - 添加 CSRF 令牌验证
   - 防止跨站请求伪造

2. **配置验证**

   - 添加配置格式验证
   - 提供友好的错误提示

3. **用户管理**

   - 支持多用户
   - 角色权限管理

4. **审计日志**

   - 记录配置变更
   - 记录登录活动

5. **性能优化**

   - 添加配置缓存
   - 规则查询优化

6. **UI 增强**
   - 添加规则编辑器
   - 可视化规则创建
   - 实时预览

## ✅ 完成状态

- ✅ Workspace 结构重组
- ✅ JWT 认证系统
- ✅ 配置管理 API
- ✅ 规则查看 API
- ✅ TLS/HTTPS 支持
- ✅ Web 前端界面
- ✅ 文档编写
- ✅ 编译测试
- ✅ 示例脚本

## 🎉 结论

WebUI 已成功实现，提供了完整的 NAT 配置管理功能。用户可以通过浏览器方便地管理配置，无需直接编辑配置文件或使用命令行。

所有代码已编译通过，可以立即投入使用。
