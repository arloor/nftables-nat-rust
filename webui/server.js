当然可以！以下是完整的 `server.js` 代码，其中包含了所有的功能和配置：

```javascript
const express = require('express');
const fs = require('fs');
const path = require('path');
const bodyParser = require('body-parser');
const cookieParser = require('cookie-parser');
const bcrypt = require('bcrypt');
const https = require('https');

const app = express();
const PORT = 3000;

// HTTPS 设置 (请提供有效的证书和私钥)
const options = {
    key: fs.readFileSync('path/to/your/private-key.pem'), 
    cert: fs.readFileSync('path/to/your/certificate.pem')
};

// 中间件
app.use(bodyParser.json());
app.use(express.static(path.join(__dirname, 'public')));
app.use(express.urlencoded({ extended: true }));
app.use(cookieParser());

// 读取和处理密码
let users = {};
fs.readFile('passwd.md', 'utf8', (err, data) => {
    if (err) {
        console.error(err);
        process.exit(1);
    }
    const lines = data.trim().split('\n');
    lines.forEach(line => {
        const [user, hashedPassword] = line.split(':');
        users[user] = hashedPassword;
    });
});

// 从 /etc/nat.conf 读取规则
let rules = [];
fs.readFile('/etc/nat.conf', 'utf8', (err, data) => {
    if (err) {
        console.error('读取配置文件失败:', err);
        return;
    }
    const lines = data.trim().split('\n');
    lines.forEach(line => {
        line = line.split('#')[0].trim(); // 移除注释
        if (!line) return; // 忽略空行
        
        const parts = line.split(',');
        const type = parts[0];
        const startPort = parts[1];
        const endPort = parts[2];
        const destination = parts[3];

        // 验证格式
        if (!type || !startPort || !destination) {
            console.error(`无效行: ${line}`);
            return;
        }

        if (type === 'SINGLE') {
            // SINGLE类型只需2个端口
            if (!endPort || isNaN(startPort)) {
                console.error(`无效的单端口转发行: ${line}`);
                return;
            }
            rules.push({ type, startPort, endPort: null, destination }); // 不需要结束端口
        } else if (type === 'RANGE') {
            // RANGE类型需要3个有效端口
            if (!endPort || isNaN(startPort) || isNaN(endPort) || Number(startPort) > Number(endPort)) {
                console.error(`范围端口不有效: ${line}`);
                return;
            }
            rules.push({ type, startPort, endPort, destination });
        } else {
            console.error(`无效类型：${type}`);
        }
    });
});

// 验证用户是否已登录
function isAuthenticated(req, res, next) {
    if (req.cookies.auth) {
        return next();
    } else {
        res.redirect('/login'); // 未登录则重定向到登录页
    }
}

// 响应根路径的GET请求
app.get('/', isAuthenticated, (req, res) => {
    res.sendFile(path.join(__dirname, 'public/index.html'));
});

// 处理登录请求
app.post('/login', async (req, res) => {
    const { username, password } = req.body;
    const hashedPassword = users[username];

    if (hashedPassword && await bcrypt.compare(password, hashedPassword)) {
        res.cookie('auth', '1'); // 设置 cookie，表示用户已认证
        res.redirect('/'); // 登录成功后重定向到主页
    } else {
        res.status(401).send('用户名或密码错误');
    }
});

// 处理登录页面的GET请求
app.get('/login', (req, res) => {
    if (req.cookies.auth) {
        return res.redirect('/'); // 已登录用户直接重定向到主页
    }
    res.sendFile(path.join(__dirname, 'public/login.html'));
});

// 处理获取规则的请求
app.get('/api/rules', (req, res) => {
    res.json(rules);
});

// 处理保存规则的请求
app.post('/save-rules', (req, res) => {
    const { rules } = req.body;
    fs.writeFile('/etc/nat.conf', rules, (err) => {
        if (err) {
            return res.status(500).json({ message: '保存规则失败' });
        }
        res.json({ message: '规则保存成功' });
    });
});

// 错误处理
app.use((err, req, res, next) => {
    console.error(err.stack);
    res.status(500).send('服务器内部发生错误！');
});

// 启动服务器
https.createServer(options, app).listen(PORT, () => {
    console.log(`服务器在 https://localhost:${PORT} 上运行`);
});
```

### 目录结构示例
为了确保代码正常工作，您可以创建以下目录结构：

```
your-project/
│
├── public/
│   ├── index.html
│   ├── login.html
│
├── path/
│   ├── to/
│       ├── your/
│           ├── private-key.pem
│           ├── certificate.pem
│
├── passwd.md
└── server.js
```

### 内容说明
- **`public/index.html`**：主页面内容。
- **`public/login.html`**：登录页面内容。
- **`passwd.md`**：包含用户名和经过 bcrypt 哈希处理的密码，以 `username:hashedPassword` 格式保存。
- **`nat.conf`**：包含 NAT 规则的配置文件。

### 运行步骤
1. 确保安装了所需的 Node.js 依赖：
   ```bash
   npm install express bcrypt
   ```
2. 运行服务器：
   ```bash
   node server.js
   ```

如果有其他问题或需要进一步的帮助，请随时告诉我！
