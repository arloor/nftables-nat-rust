# WebUI 使用示例

本文档展示如何使用 WebUI 管理 NAT 转发规则。

## 基本使用流程

### 1. 启动 WebUI

使用提供的启动脚本：

```bash
./start-webui.sh
```

或手动启动：

```bash
./target/release/webui \
  --port 8443 \
  --username admin \
  --password your_password \
  --toml-config /etc/nat.toml \
  --cert cert.pem \
  --key key.pem
```

### 2. 访问 WebUI

在浏览器中访问：

```
https://your-server-ip:8443
```

如果使用自签名证书，浏览器会提示安全警告，点击"继续访问"即可。

### 3. 登录

使用启动时设置的用户名和密码登录。

### 4. 管理配置

#### 查看配置

在"配置管理"标签页中，可以看到当前的配置文件内容。

#### 编辑配置

##### TOML 格式示例

```json
{
  "rules": [
    {
      "type": "single",
      "sport": 10000,
      "dport": 443,
      "domain": "example.com",
      "protocol": "all",
      "ip_version": "both",
      "comment": "HTTPS 转发"
    },
    {
      "type": "range",
      "portStart": 20000,
      "portEnd": 20100,
      "domain": "example.com",
      "protocol": "tcp",
      "ip_version": "ipv4"
    }
  ]
}
```

##### 传统格式示例

```
# 单端口转发
SINGLE,10000,443,example.com,all
# 端口段转发
RANGE,20000,20100,example.com,tcp
# 本地重定向
REDIRECT,8080,3128,tcp
```

#### 保存配置

编辑完成后，点击"保存配置"按钮保存更改。

⚠️ **注意**：保存配置后，需要重启 NAT 服务才能使配置生效：

```bash
systemctl restart nat
```

### 5. 查看规则

在"规则查看"标签页中：

1. 点击"刷新规则"按钮
2. 查看当前生效的 nftables 规则

这里显示的是实际运行中的 NAT 规则，包括：

- IPv4 NAT 表（table ip self-nat）
- IPv6 NAT 表（table ip6 self-nat）

## 安全最佳实践

### 1. 使用强密码

```bash
# 生成随机密码
openssl rand -base64 20
```

### 2. 使用可信证书

生产环境建议使用 Let's Encrypt 证书：

```bash
# 安装 certbot
apt install certbot  # Debian/Ubuntu
yum install certbot  # CentOS/RHEL

# 获取证书
certbot certonly --standalone -d yourdomain.com
```

然后使用证书启动：

```bash
./target/release/webui \
  --cert /etc/letsencrypt/live/yourdomain.com/fullchain.pem \
  --key /etc/letsencrypt/live/yourdomain.com/privkey.pem \
  ...
```

### 3. 限制访问

使用防火墙限制访问：

```bash
# 只允许特定 IP 访问 WebUI
nft add rule inet filter input ip saddr 192.168.1.100 tcp dport 8443 accept
nft add rule inet filter input tcp dport 8443 drop
```

### 4. 使用反向代理

通过 Nginx 添加额外的安全层：

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

## 故障排查

### 无法访问 WebUI

1. 检查服务是否运行：

   ```bash
   ps aux | grep webui
   ```

2. 检查端口是否监听：

   ```bash
   netstat -tuln | grep 8443
   ```

3. 检查防火墙规则：
   ```bash
   nft list ruleset | grep 8443
   ```

### 配置保存失败

1. 检查配置文件权限：

   ```bash
   ls -l /etc/nat.toml
   chmod 644 /etc/nat.toml
   ```

2. 检查 JSON 格式是否正确
3. 查看浏览器控制台错误信息

### 规则不生效

1. 保存配置后重启 NAT 服务：

   ```bash
   systemctl restart nat
   ```

2. 检查 NAT 服务日志：
   ```bash
   journalctl -u nat -f
   ```

## systemd 集成

创建 WebUI 的 systemd 服务：

```bash
cat > /etc/systemd/system/nat-webui.service <<EOF
[Unit]
Description=NAT WebUI Management Interface
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=/opt/nftables-nat-rust
ExecStart=/opt/nftables-nat-rust/target/release/webui \\
  --port 8443 \\
  --username admin \\
  --password $(cat /etc/nat-webui-password) \\
  --jwt-secret $(cat /etc/nat-webui-jwt-secret) \\
  --toml-config /etc/nat.toml \\
  --cert /etc/ssl/certs/nat-webui.crt \\
  --key /etc/ssl/private/nat-webui.key
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

# 存储敏感信息
echo "your_password_here" > /etc/nat-webui-password
echo "your_jwt_secret_here" > /etc/nat-webui-jwt-secret
chmod 600 /etc/nat-webui-password /etc/nat-webui-jwt-secret

# 启动服务
systemctl daemon-reload
systemctl enable nat-webui
systemctl start nat-webui
```
