# WebUI for nftables-nat-rust

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
cargo build --release --package webui
```

### 运行

#### HTTP 模式（开发环境）

```bash
./target/release/webui \
  --port 8080 \
  --username admin \
  --password your_password \
  --toml-config /path/to/nat.toml
```

#### HTTPS 模式（生产环境）

```bash
./target/release/webui \
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

1. **访问 WebUI**

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

## 安全建议

⚠️ **生产环境必须使用 HTTPS！**

1. **使用强密码**

   - 密码长度至少 12 位
   - 包含大小写字母、数字和特殊字符

2. **更改默认 JWT 密钥**

   - 使用长度至少 32 字节的随机字符串

   ```bash
   --jwt-secret "$(openssl rand -base64 32)"
   ```

3. **使用有效的 TLS 证书**

   - 生产环境使用 Let's Encrypt 等机构颁发的证书
   - 不要使用自签名证书

4. **限制访问**
   - 使用防火墙限制只允许特定 IP 访问
   - 考虑使用反向代理（如 Nginx）添加额外安全层

## systemd 服务示例

创建 `/etc/systemd/system/nat-webui.service`:

```ini
[Unit]
Description=NAT WebUI Management Interface
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=/opt/nftables-nat-rust
ExecStart=/opt/nftables-nat-rust/target/release/webui \
  --port 8443 \
  --username admin \
  --password YOUR_STRONG_PASSWORD \
  --jwt-secret YOUR_JWT_SECRET \
  --toml-config /etc/nftables-nat/nat.toml \
  --cert /etc/ssl/certs/nat-webui.crt \
  --key /etc/ssl/private/nat-webui.key
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

启动服务：

```bash
systemctl daemon-reload
systemctl enable nat-webui
systemctl start nat-webui
systemctl status nat-webui
```

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

## 故障排查

### WebUI 无法启动

1. 检查端口是否被占用

   ```bash
   netstat -tuln | grep 8080
   ```

2. 检查证书文件权限
   ```bash
   ls -l /path/to/cert.pem /path/to/key.pem
   ```

### 无法登录

1. 检查用户名和密码是否正确
2. 检查浏览器控制台是否有错误
3. 检查服务器日志

### 配置保存失败

1. 检查配置文件权限
2. 检查配置格式是否正确
3. 检查磁盘空间

## 开发

### 项目结构

```
webui/
├── Cargo.toml
├── src/
│   ├── main.rs        # 入口文件
│   ├── server.rs      # 服务器配置
│   ├── handlers.rs    # 请求处理
│   ├── auth.rs        # JWT 认证
│   └── config.rs      # 配置管理
└── static/
    ├── login.html     # 登录页面
    └── index.html     # 主界面
```

### 本地开发

```bash
# 开发模式运行
RUST_LOG=debug cargo run --package webui -- \
  --port 8080 \
  --username admin \
  --password admin \
  --toml-config nat.toml
```

## License

与主项目相同
