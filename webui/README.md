这个项目完整的代码，包含 `bcryptTool.js`、`server.js`、`public/index.html` 和 `public/admin.html` 文件。

### 项目结构

```
nftables-nat-rust-webui/
├── bcryptTool.js
├── passwd.md
├── server.js
└── public/
    ├── index.html
    └── admin.html
```

### 1. `bcryptTool.js`

```javascript
const bcrypt = require('bcrypt');
const fs = require('fs');
const readline = require('readline');

// 创建一个接口读取命令行输入
const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout
});

const passwdFilePath = 'passwd.md';

// 加密密码
const encryptPassword = async (plainPassword) => {
    const saltRounds = 10;
    return await bcrypt.hash(plainPassword, saltRounds);
};

// 写入文件
const writeToFile = (entry) => {
    fs.appendFile(passwdFilePath, entry + '\n', (err) => {
        if (err) {
            console.error('写入文件失败:', err);
        } else {
            console.log('密码已成功加密并写入到 passwd.md 文件。');
        }
        rl.close();
    });
};

// 解密密码
const checkPassword = async (providedPassword, storedHash) => {
    const match = await bcrypt.compare(providedPassword, storedHash);
    if (match) {
        console.log('密码匹配！');
    } else {
        console.log('密码不匹配！');
    }
    rl.close();
};

// 查询用户列表
const listUsers = () => {
    fs.readFile(passwdFilePath, 'utf8', (err, data) => {
        if (err) {
            console.error('读取文件失败:', err);
            rl.close();
            return;
        }
        const lines = data.trim().split('\n');
        console.log("用户列表：");
        lines.forEach(line => console.log(line.split(':')[0])); // 展示用户名
        rl.close();
    });
};

// 删除用户
const deleteUser = () => {
    rl.question('请输入要删除的用户名: ', (username) => {
        fs.readFile(passwdFilePath, 'utf8', (err, data) => {
            if (err) {
                console.error('读取文件失败:', err);
                rl.close();
                return;
            }
            const lines = data.trim().split('\n').filter(line => !line.startsWith(username + ':'));
            if (lines.length === data.trim().split('\n').length) {
                console.log('用户不存在！');
            } else {
                fs.writeFile(passwdFilePath, lines.join('\n'), (err) => {
                    if (err) {
                        console.error('删除用户失败:', err);
                    } else {
                        console.log('用户已成功删除。');
                    }
                });
            }
            rl.close();
        });
    });
};

// 启动工具
const startTool = () => {
    console.log("选择一个选项：");
    console.log("1. 加密新密码并写入 passwd.md");
    console.log("2. 验证密码");
    console.log("3. 查询用户列表");
    console.log("4. 删除用户");

    rl.question('请输入选项 (1/2/3/4): ', async (choice) => {
        if (choice === '1') {
            rl.question('请输入要加密的密码: ', async (plainPassword) => {
                const hashedPassword = await encryptPassword(plainPassword);
                rl.question('请输入用户名: ', (username) => {
                    const entry = `${username}:${hashedPassword}`;
                    writeToFile(entry);
                });
            });
        } else if (choice === '2') {
            rl.question('请输入用户名: ', (username) => {
                rl.question('请输入要验证的密码: ', async (providedPassword) => {
                    fs.readFile(passwdFilePath, 'utf8', (err, data) => {
                        if (err) {
                            console.error('读取文件失败:', err);
                            rl.close();
                            return;
                        }
                        const lines = data.trim().split('\n');
                        const userEntry = lines.find(line => line.startsWith(username + ':'));

                        if (userEntry) {
                            const storedHash = userEntry.split(':')[1];
                            checkPassword(providedPassword, storedHash);
                        } else {
                            console.log('用户不存在！');
                            rl.close();
                        }
                    });
                });
            });
        } else if (choice === '3') {
            listUsers();
        } else if (choice === '4') {
            deleteUser();
        } else {
            console.log('无效的选项！');
            rl.close();
        }
    });
};

// 启动工具
startTool();
```

### 2. `server.js`

```javascript
// server.js
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
    key: fs.readFileSync('/root/nftables-nat-rust-webui/ssl/private-key.pem'),
    cert: fs.readFileSync('/root/nftables-nat-rust-webui/ssl/certificate.pem')
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
const readRulesFile = () => {
    fs.readFile('/etc/nat.conf', 'utf8', (err, data) => {
        if (err) {
            console.error('读取配置文件失败:', err);
            return;
        }
        rules = data.trim().split('\n').map(line => {
            line = line.split('#')[0].trim(); // 移除注释
            return line ? line.split(',') : null;
        }).filter(Boolean).map(parts => {
            return {
                type: parts[0],
                startPort: parts[1],
                endPort: parts[2] || null,
                destination: parts[3],
                protocol: parts[4] || null // 新增协议字段
            };
        });
    });
};
readRulesFile();

// 身份验证中间件
function isAuthenticated(req, res, next) {
    if (req.cookies.auth) {
        return next();
    } else {
        res.redirect('/index');
    }
}

// 路由: 登录页面
app.get('/index', (req, res) => {
    if (req.cookies.auth) {
        return res.redirect('/admin');
    }
    res.sendFile(path.join(__dirname, 'public/index.html'));
});

// 路由: 后台管理，需身份验证
app.get('/admin', isAuthenticated, (req, res) => {
    res.sendFile(path.join(__dirname, 'public/admin.html'));
});

// 路由: 登录请求处理
app.post('/login', async (req, res) => {
    const { username, password } = req.body;
    const hashedPassword = users[username];

    if (hashedPassword && await bcrypt.compare(password, hashedPassword)) {
        res.cookie('auth', '1'); // 设置cookie
        res.redirect('/admin');
    } else {
        res.status(401).send('用户名或密码错误');
    }
});

// 其他需要身份验证的路由
app.get('/api/rules', isAuthenticated, (req, res) => {
    res.json(rules);
});

app.post('/edit-rule', isAuthenticated, (req, res) => {
    const { index, startPort, endPort, destination, protocol } = req.body;
    if (index < 0 || index >= rules.length) {
        return res.status(400).json({ message: '无效的规则索引' });
    }

    rules[index] = {
        type: rules[index].type,
        startPort,
        endPort,
        destination,
        protocol // 更新协议
    };
    res.json({ message: '规则编辑成功' });
});

// 处理保存规则的请求
app.post('/save-rules', isAuthenticated, (req, res) => {
    const rulesData = req.body.rules.map(rule => {
        const endPort = rule.endPort || rule.startPort; // 处理空值
        const protocol = rule.protocol || ''; // 获取协议

        return `${rule.type},${rule.startPort},${endPort},${rule.destination}${protocol ? ',' + protocol : ''}`;
    }).join('\n');

    fs.writeFile('/etc/nat.conf', rulesData, (err) => {
        if (err) {
            return res.status(500).json({ message: '保存规则失败' });
        }
        readRulesFile(); // 重新加载规则
        res.json({ message: '规则保存成功' });
    });
});

// 登出
app.post('/logout', (req, res) => {
    res.clearCookie('auth'); // 清除cookie
    res.redirect('/index'); // 重定向到登录页面
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

### 3. `public/index.html`

```html
<!DOCTYPE html>
<html lang="zh">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>登录</title>
    <style>
        * {
            box-sizing: border-box;
        }
        body {
            font-family: "Helvetica Neue", Arial, sans-serif;
            background: linear-gradient(135deg, #e0f7fa 0%, #b2ebf2 100%);
            display: flex;
            justify-content: center;
            align-items: center;
            height: 100vh;
            margin: 0;
            padding: 0 20px;
        }
        .login-container {
            background: #ffffff;
            border-radius: 12px;
            padding: 40px;
            box-shadow: 0 4px 30px rgba(0, 0, 0, 0.1);
            width: 100%;
            max-width: 400px;
            text-align: center;
            animation: fadeIn 0.5s ease-in-out;
        }
        @keyframes fadeIn {
            from {
                opacity: 0;
            }
            to {
                opacity: 1;
            }
        }
        h2 {
            margin-bottom: 30px;
            color: #007aff;
            font-size: 28px;
        }
        input {
            width: 100%;
            padding: 15px;
            margin-bottom: 15px;
            border: 1px solid #007aff;
            border-radius: 5px;
            font-size: 16px;
            transition: border 0.3s ease;
        }
        input:focus {
            border-color: #0051a8;
            outline: none;
        }
        button {
            width: 100%;
            padding: 15px;
            background-color: #007aff;
            color: #ffffff;
            border: none;
            border-radius: 5px;
            font-size: 16px;
            cursor: pointer;
            transition: background-color 0.3s ease;
        }
        button:hover {
            background-color: #0051a8;
        }
        .message {
            margin-top: 15px;
            color: #d9534f; /* 红色错误信息 */
            display: none; /* 默认隐藏 */
        }
        footer {
            margin-top: 20px;
            font-size: 14px;
            color: #666;
        }
        @media (max-width: 480px) {
            .login-container {
                padding: 30px;
            }
            h2 {
                font-size: 24px;
            }
            input, button {
                font-size: 14px;
            }
        }
    </style>
</head>
<body>

    <div class="login-container">
        <h2>用户登录</h2>
        <form id="loginForm" onsubmit="return login(event)">
            <input type="text" id="username" placeholder="用户名" required>
            <input type="password" id="password" placeholder="密码" required>
            <button type="submit">登录</button>
        </form>
        <div class="message" id="message"></div>
        <footer>
            <p>© JiangChu. All Rights Reserved</p>
        </footer>
    </div>

    <script>
        async function login(event) {
            event.preventDefault();
            const username = document.getElementById('username').value;
            const password = document.getElementById('password').value;
            const messageDiv = document.getElementById('message');

            const response = await fetch('/login', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({ username, password }),
            });

            if (response.ok) {
                window.location.href = '/admin'; // 成功登录后重定向到后台管理
            } else {
                messageDiv.innerHTML = '用户名或密码错误！';
                messageDiv.style.display = 'block'; // 显示错误信息
            }
        }
    </script>
</body>
</html>
```

### 4. `public/admin.html`

```html
<!DOCTYPE html>
<html lang="zh">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>端口转发控制台</title>
    <link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/5.15.4/css/all.min.css">
    <style>
        * {
            box-sizing: border-box;
        }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Helvetica Neue', 'Arial', sans-serif;
            background: linear-gradient(135deg, #e0f7fa 0%, #b2ebf2 100%);
            color: #333;
            margin: 0;
            padding: 20px;
        }

        .container {
            max-width: 600px;
            margin: auto;
            padding: 30px;
            background: #fff;
            border-radius: 12px;
            box-shadow: 0 4px 20px rgba(0, 0, 0, 0.2);
        }

        h1, h2 {
            text-align: center;
            color: #007aff;
        }

        label {
            display: block;
            margin-top: 20px;
            font-weight: bold;
        }

        input[type="text"],
        select {
            width: calc(100% - 22px);
            padding: 12px;
            margin: 8px 0;
            border: 1px solid #bbb;
            border-radius: 6px;
            font-size: 16px;
            transition: border-color 0.3s;
        }

        input[type="text"]:focus,
        select:focus {
            border-color: #007aff;
        }

        input[type="button"] {
            width: 100%;
            padding: 14px;
            background-color: #007aff;
            color: white;
            border: none;
            border-radius: 6px;
            cursor: pointer;
            font-size: 16px;
            transition: background-color 0.3s, transform 0.2s;
        }

        input[type="button"]:hover {
            background-color: #0051a8;
            transform: translateY(-2px);
        }

        .message {
            display: none;
            padding: 10px;
            margin-top: 15px;
            border-radius: 5px;
            color: #fff;
        }

        .success {
            background-color: #5cb85c;
        }

        .error {
            background-color: #d9534f;
        }

        table {
            width: 100%;
            border-collapse: collapse;
            margin-top: 20px;
            border-radius: 10px;
            overflow: hidden;
            box-shadow: 0 2px 10px rgba(0, 0, 0, 0.1);
        }

        th,
        td {
            padding: 12px;
            border: 1px solid #ccc;
            text-align: center;
        }

        th {
            background-color: #f1f1f1;
            font-weight: bold;
        }

        tr:nth-child(even) {
            background-color: #f9f9f9;
        }

        tr:hover {
            background-color: #f0f0f0;
        }

        pre {
            background-color: #f9f9f9;
            padding: 15px;
            border-radius: 6px;
            overflow-x: auto;
            font-family: monospace;
            white-space: pre-wrap;
        }

        footer {
            text-align: center;
            margin-top: 20px;
            font-size: 0.8em;
            color: #777;
        }
    </style>
</head>

<body>
    <div class="container">
        <h1>端口转发控制台</h1>
        <h2>添加新规则</h2>
        
        <label>规则类型:
            <select id="ruleType">
                <option value="SINGLE">SINGLE</option>
                <option value="RANGE">RANGE</option>
            </select>
        </label>
        <div class="note" id="note">目标端口为空则默认自动填入和本机端口一样的端口</div>
        
        <input type="text" id="startPort" placeholder="起始端口" required>
        <div class="note">类型选择SINGLE时视同为本地端口</div>
        
        <input type="text" id="endPort" placeholder="结束端口" required>
        <div class="note">类型选择SINGLE时视同为目标端口</div>
        
        <input type="text" id="destination" placeholder="目标域名或IP" required>
        <div class="note">转发到本地：行尾域名处填写localhost即可</div>
        
        <input type="text" id="protocol" placeholder="tcp/udp（可选）">
        <div class="note">留空默认为全部转发，仅需tcp则填写tcp</div>

        <input type="button" value="添加规则" onclick="addRule()">
        <div class="message" id="message"></div>
        
        <h2>当前规则</h2>
        <table id="rulesTable">
            <thead>
                <tr>
                    <th>规则类型</th>
                    <th>起始端口</th>
                    <th>结束端口</th>
                    <th>目标</th>
                    <th>协议</th>
                    <th>操作</th>
                </tr>
            </thead>
            <tbody>
                <!-- 规则将被动态插入这里 -->
            </tbody>
        </table>
        <input type="button" value="保存规则" onclick="saveRules()">

        <h2>配置预览（/etc/nat.conf）</h2>
        <pre id="rulesPreview"></pre>
        <input type="button" value="登出" onclick="logout()">
    </div>

    <footer>
        <p>© JiangChu. All Rights Reserved.</p>
    </footer>

    <script>
        let rules = [];
        
        window.onload = async () => {
            await fetchAndRenderRules();
        };

        async function fetchAndRenderRules() {
            const response = await fetch('/api/rules');
            const fetchedRules = await response.json();
            rules = fetchedRules; // 注意这里直接赋值
            renderRules();
            updatePreview();
        }

        function addRule() {
            const startPort = document.getElementById('startPort').value;
            const endPort = document.getElementById('endPort').value || startPort;
            const destination = document.getElementById('destination').value;
            const ruleType = document.getElementById('ruleType').value;
            const protocol = document.getElementById('protocol').value;

            if (startPort && destination) {
                rules.push({ type: ruleType, startPort, endPort, destination, protocol });
                renderRules();
                clearInputs();
                updatePreview();
                showMessage('规则添加成功！', 'success');
            } else {
                showMessage('请填写所有必需的字段！', 'error');
            }
        }

        function renderRules() {
            const tableBody = document.querySelector('#rulesTable tbody');
            tableBody.innerHTML = '';
            rules.forEach((rule, index) => {
                const newRow = tableBody.insertRow();
                newRow.insertCell(0).innerText = rule.type;
                newRow.insertCell(1).innerText = rule.startPort;
                newRow.insertCell(2).innerText = rule.endPort;
                newRow.insertCell(3).innerText = rule.destination;
                newRow.insertCell(4).innerText = rule.protocol || '未指定';

                const editCell = newRow.insertCell(5);
                editCell.innerHTML = `<button onclick="editRule(${index})">编辑</button> <button onclick="deleteRule(${index})">删除</button>`;
            });
        }

        function editRule(index) {
            const rule = rules[index];
            document.getElementById('startPort').value = rule.startPort;
            document.getElementById('endPort').value = rule.endPort;
            document.getElementById('destination').value = rule.destination;
            document.getElementById('protocol').value = rule.protocol || '';
            document.getElementById('ruleType').value = rule.type;
            deleteRule(index);
        }

        function deleteRule(index) {
            rules.splice(index, 1);
            renderRules();
            updatePreview();
            showMessage('规则已删除', 'success');
        }

        function clearInputs() {
            document.getElementById('startPort').value = '';
            document.getElementById('endPort').value = '';
            document.getElementById('destination').value = '';
            document.getElementById('protocol').value = '';
        }

        function updatePreview() {
            const preview = document.getElementById('rulesPreview');
            const previewText = rules.map(rule => `${rule.type},${rule.startPort},${rule.endPort || rule.startPort},${rule.destination}${rule.protocol ? ',' + rule.protocol : ''}`).join('\n');
            preview.innerText = previewText;
        }

        async function saveRules() {
            const response = await fetch('/save-rules', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ rules })
            });

            if (response.ok) {
                showMessage('规则保存成功！', 'success');
            } else {
                showMessage('规则保存失败，请重试。', 'error');
            }
        }

        async function logout() {
            await fetch('/logout', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
            });
            window.location.href = '/';
        }

        function showMessage(message, type) {
            const messageDiv = document.getElementById('message');
            messageDiv.innerText = message;
            messageDiv.className = `message ${type}`;
            messageDiv.style.display = 'block';
            setTimeout(() => {
                messageDiv.style.display = 'none';
            }, 3000); // 3秒后隐藏消息
        }
    </script>
</body>
</html>
```

### 注意事项

1. **证书路径**：请在 `server.js` 中替换 `/root/nftables-nat-rust-webui/ssl/private-key.pem` 和 `/root/nftables-nat-rust-webui/ssl/certificate.pem` 为您的 SSL 证书和私钥的实际路径。
2. **读取权限**：确保 Node.js 进程对 `/etc/nat.conf` 和 `passwd.md` 的读取权限。
3. **密码文件**：用户名和密码可以用`bcryptTool.js`生成，确保 `passwd.md` 文件的格式为 `用户名:哈希密码`，例如 `admin:$2b$10$gY9KnYXxJ.PqybUkf0z2y.VD2LZX1X5LfKoJu9zW0PzW.q34654eO`。`更改或者新增之后需要重启程序。`
4. **安装依赖**：确保已用以下命令安装需要的 npm 包：
    ```bash
    npm init -y && npm install express bcrypt cookie-parser body-parser fs https
    ```
5. **启动服务器**：使用以下命令启动服务器：
    ```bash
    node server.js
    ```

### 要设置 Node.js 应用在开机时自动启动，您可以使用 `systemd` 来创建一个服务。下面是如何在 Linux 系统（如 Ubuntu）中进行设置的步骤。

### 1. 创建 Node.js 应用的服务文件

1. **打开终端**并使用您喜欢的文本编辑器创建一个新的服务文件，例如 `/etc/systemd/system/nftables-nat-rust-webui.service`（将 `myapp` 替换为您的应用名称）：

```bash
sudo nano /etc/systemd/system/nftables-nat-rust-webui.service
```

2. **将以下内容复制并粘贴到该文件中**，并根据实际情况修改路径和应用名称：

```ini
[Unit]
Description=nftables-nat-rust-webui Node.js Application
After=network.target

[Service]
ExecStart=/usr/bin/node /root/nftables-nat-rust-webui/server.js
WorkingDirectory=/root/nftables-nat-rust-webui
Restart=always
# User and Group settings
User=root
Group=root

[Install]
WantedBy=multi-user.target
```

### 2. 参数说明

- **ExecStart**：Node.js 可执行文件的路径，以及您应用的路径（`/root/nftables-nat-rust-webui/server.js` 应替换成您的实际路径）。
- **WorkingDirectory**：您的 Node.js 应用的根目录。
- **User** 和 **Group**：运行该服务的用户和组，确保该用户有权限访问应用目录。
- **Restart**：当服务因为错误而停止时，`systemd` 将自动重新启动服务。

### 3. 重新加载 `systemd` 管理器配置

运行以下命令以重新加载 `systemd` 以使新服务文件生效：

```bash
sudo systemctl daemon-reload
```

### 4. 启动服务

使用以下命令启动服务：

```bash
sudo systemctl start nftables-nat-rust-webui
```

### 5. 设置服务开机自启动

要启用服务在启动时自动运行，请运行：

```bash
sudo systemctl enable nftables-nat-rust-webui
```

### 6. 检查服务状态

可以使用以下命令检查服务状态：

```bash
sudo systemctl status nftables-nat-rust-webui
```

这将显示服务的当前状态，以确保它正在运行。

### 7. 查看日志

您可以通过以下命令查看服务的日志输出，以帮助调试：

```bash
journalctl -u nftables-nat-rust-webui
```

### 总结

以上步骤将帮助您在 Linux 系统上设置 Node.js 应用的开机自启动。如果您的系统不是使用 `systemd`，可能需要使用其他方式进行配置，具体取决于您的 Linux 发行版（例如使用 `init.d` 或 `upstart`）。
