# 🚀 WebUI 快速开始指南

本指南帮助你在 5 分钟内启动 NAT 管理 WebUI。

## 📋 前提条件

1. 已安装 Rust 工具链（1.70+）
2. 系统已安装 nftables
3. 有一个 NAT 配置文件（TOML 或传统格式）

## ⚡ 快速启动（开发/测试）

### 1. 克隆并编译

```bash
git clone https://github.com/arloor/nftables-nat-rust.git
cd nftables-nat-rust
cargo build --release --package webui
```

### 2. 创建测试配置

如果你还没有配置文件，创建一个测试配置：

```bash
cat > test-nat.toml <<EOF
[[rules]]
type = "single"
sport = 10000
dport = 443
domain = "example.com"
protocol = "all"
ip_version = "both"
comment = "测试规则"
EOF
```

### 3. 启动 WebUI（HTTP 模式）

```bash
./target/release/webui \
  --port 8080 \
  --username admin \
  --password admin123 \
  --toml-config test-nat.toml
```

### 4. 访问界面

打开浏览器访问：

```
http://localhost:8080
```

登录信息：

- 用户名: `admin`
- 密码: `admin123`

## 🔒 生产环境部署（HTTPS）

### 1. 生成 TLS 证书

#### 方式 A：自签名证书（测试用）

```bash
openssl req -x509 -newkey rsa:4096 -nodes \
  -keyout key.pem \
  -out cert.pem \
  -days 365 \
  -subj "/CN=localhost"
```

#### 方式 B：Let's Encrypt（推荐）

```bash
# 安装 certbot
apt install certbot  # Debian/Ubuntu
yum install certbot  # CentOS/RHEL

# 获取证书
certbot certonly --standalone -d yourdomain.com

# 证书路径
CERT=/etc/letsencrypt/live/yourdomain.com/fullchain.pem
KEY=/etc/letsencrypt/live/yourdomain.com/privkey.pem
```

### 2. 生成强密码和 JWT 密钥

```bash
# 生成密码
PASSWORD=$(openssl rand -base64 20)
echo "密码: $PASSWORD"

# 生成 JWT 密钥
JWT_SECRET=$(openssl rand -base64 32)
```

### 3. 启动 HTTPS 服务

```bash
./target/release/webui \
  --port 8443 \
  --username admin \
  --password "$PASSWORD" \
  --jwt-secret "$JWT_SECRET" \
  --toml-config /etc/nat.toml \
  --cert cert.pem \
  --key key.pem
```

### 4. 访问

```
https://your-server-ip:8443
```

## 🔧 使用 systemd 管理

### 1. 创建密码文件

```bash
# 创建存储密码的文件
echo "your_strong_password" > /etc/nat-webui-password
echo "your_jwt_secret" > /etc/nat-webui-jwt-secret
chmod 600 /etc/nat-webui-password /etc/nat-webui-jwt-secret
```

### 2. 创建 systemd 服务

```bash
cat > /etc/systemd/system/nat-webui.service <<'EOF'
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
  --password $(cat /etc/nat-webui-password) \
  --jwt-secret $(cat /etc/nat-webui-jwt-secret) \
  --toml-config /etc/nat.toml \
  --cert /etc/ssl/certs/nat-webui.crt \
  --key /etc/ssl/private/nat-webui.key
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF
```

### 3. 启动服务

```bash
# 复制程序到 /opt
cp -r /path/to/nftables-nat-rust /opt/

# 启动服务
systemctl daemon-reload
systemctl enable nat-webui
systemctl start nat-webui

# 查看状态
systemctl status nat-webui

# 查看日志
journalctl -u nat-webui -f
```

## 📖 常见操作

### 编辑配置

1. 登录 WebUI
2. 点击"配置管理"标签
3. 编辑配置内容（JSON 格式）
4. 点击"保存配置"

**重要**：保存配置后需要重启 NAT 服务使其生效：

```bash
systemctl restart nat
```

### 查看规则

1. 点击"规则查看"标签
2. 点击"刷新规则"按钮
3. 查看当前生效的 nftables 规则

### 修改密码

重启 WebUI 时使用新密码即可：

```bash
# 停止服务
systemctl stop nat-webui

# 更新密码文件
echo "new_password" > /etc/nat-webui-password

# 重启服务
systemctl start nat-webui
```

## 🛡️ 安全建议

### ⚠️ 必做项

1. **使用 HTTPS**

   - 生产环境必须启用 TLS
   - 使用可信证书（Let's Encrypt）

2. **强密码**

   - 至少 16 位
   - 包含大小写字母、数字、特殊字符
   - 使用密码管理器

3. **更换 JWT 密钥**
   - 不要使用默认值
   - 使用随机生成的密钥

### 🔐 可选项

1. **防火墙限制**

```bash
# 只允许特定 IP
nft add rule inet filter input ip saddr 192.168.1.0/24 tcp dport 8443 accept
nft add rule inet filter input tcp dport 8443 drop
```

2. **Nginx 反向代理**

```nginx
server {
    listen 443 ssl http2;
    server_name nat.example.com;

    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

## 🐛 故障排查

### 无法访问 WebUI

```bash
# 检查服务状态
systemctl status nat-webui

# 检查端口监听
netstat -tuln | grep 8443

# 查看日志
journalctl -u nat-webui -n 50
```

### 登录失败

1. 检查用户名和密码是否正确
2. 查看浏览器控制台是否有错误
3. 检查服务器日志

### 配置保存失败

```bash
# 检查配置文件权限
ls -l /etc/nat.toml
chmod 644 /etc/nat.toml

# 检查磁盘空间
df -h
```

## 📚 更多资源

- [完整文档](webui/README.md)
- [使用教程](WEBUI_USAGE.md)
- [实现总结](.github/IMPLEMENTATION_SUMMARY.md)
- [主项目文档](README.md)

## 💡 小贴士

1. **开发环境**使用 HTTP 模式更方便
2. **生产环境**必须使用 HTTPS
3. 配置修改后记得重启 NAT 服务
4. 定期备份配置文件
5. 使用 systemd 管理服务更可靠

## 🎉 完成！

现在你已经成功启动了 NAT 管理 WebUI！

如有问题，请查看日志或提交 Issue。
