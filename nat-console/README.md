# nat-console for nftables-nat-rust

Web 管理界面，用于管理 nftables NAT 转发规则。

## 功能特性

- 🔐 基于 JWT 的登录认证
- 🔒 支持 HTTPS/TLS 加密传输
- 📝 可视化编辑 NAT 配置文件
- 📋 实时查看 nftables 规则
- 🔄 支持传统配置格式和 TOML 格式

## 快速开始

### 编译

```bash
cd /root/nftables-nat-rust
cargo build --release --package nat-console
```

### 运行

#### HTTP 模式（开发环境）

```bash
./target/release/nat-console \
  --port 8080 \
  --username admin \
  --password your_password \
  --toml-config /path/to/nat.toml
```

#### HTTPS 模式（生产环境）

```bash
./target/release/nat-console \
  --port 8443 \
  --username admin \
  --password your_password \
  --toml-config /path/to/nat.toml \
  --cert /path/to/cert.pem \
  --key /path/to/key.pem
```

### 生成自签名证书（用于测试）

```bash
openssl req -x509 -newkey rsa:4096 -nodes \
  -keyout key.pem \
  -out cert.pem \
  -days 365 \
  -subj "/CN=localhost"
```

## 命令行参数

| 参数                  | 说明              | 必需             | 默认值                               |
| --------------------- | ----------------- | ---------------- | ------------------------------------ |
| `--port, -p`          | 监听端口          | 否               | 8080                                 |
| `--username, -u`      | 登录用户名        | 是               | -                                    |
| `--password`          | 登录密码          | 是               | -                                    |
| `--jwt-secret`        | JWT 密钥          | 否               | your-secret-key-change-in-production |
| `--cert`              | TLS 证书路径      | 否（HTTPS 需要） | -                                    |
| `--key`               | TLS 私钥路径      | 否（HTTPS 需要） | -                                    |
| `--toml-config`       | TOML 配置文件路径 | 二选一           | -                                    |
| `--compatible-config` | 传统配置文件路径  | 二选一           | -                                    |

## 使用说明

1. **访问 nat-console**

   - HTTP: `http://your-server:8080`
   - HTTPS: `https://your-server:8443`

2. **登录**

   - 使用启动时指定的用户名和密码登录

3. **编辑配置**

   - 在"配置管理"标签页中编辑配置文件
   - 点击"保存配置"按钮保存更改

4. **查看规则**
   - 在"规则查看"标签页中查看当前生效的 nftables 规则
   - 点击"刷新规则"按钮获取最新规则

## API 接口

### 登录

```bash
POST /api/login
Content-Type: application/json

{
  "username": "admin",
  "password": "your_password"
}
```

### 获取配置

```bash
GET /api/config
Cookie: token=<jwt_token>
```

### 保存配置

```bash
POST /api/config
Cookie: token=<jwt_token>
Content-Type: application/json

{
  "format": "toml",
  "content": { ... }
}
```

### 获取规则

```bash
GET /api/rules
Cookie: token=<jwt_token>
```

### 退出登录

```bash
POST /api/logout
Cookie: token=<jwt_token>
```

## License

与主项目相同
