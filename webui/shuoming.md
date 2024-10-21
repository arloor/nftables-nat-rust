完整的代码，包含 `bcryptTool.js`、`server.js`、`public/index.html` 和 `public/login.html` 文件。
### 项目结构
```
nftables-nat-rust-webui/
├── bcryptTool.js
├── passwd.md
├── server.js
└── public/
    ├── index.html
    └── login.html
```

### 1. `bcryptTool.js`

这是一个工具文件，用于处理密码的加密和验证。

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

// 启动工具
const startTool = () => {
    console.log("选择一个选项：");
    console.log("1. 加密新密码并写入 passwd.md");
    console.log("2. 验证密码");

    rl.question('请输入选项 (1/2): ', async (choice) => {
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

主要服务器文件，处理请求和逻辑。

```javascript
const express = require('express');
const fs = require('fs');
const path = require('path');
const bodyParser = require('body-parser');
const cookieParser = require('cookie-parser');
const bcrypt = require('bcrypt');
const https = require('https');
const bcryptTool = require('./bcryptTool');

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

        if (type !== 'SINGLE' && type !== 'RANGE') {
            console.error(`无效类型：${type}`);
            return;
        }

        if (type === 'SINGLE' && (!endPort || endPort !== '-')) {
            console.error(`单端口转发的结束端口必须为'-'，在行: ${line}`);
            return;
        }

        if (type === 'RANGE') {
            // 检查端口范围的有效性
            if (!endPort || isNaN(startPort) || isNaN(endPort) || Number(startPort) > Number(endPort)) {
                console.error(`范围端口不有效: ${line}`);
                return;
            }
        }

        rules.push({ type, startPort, endPort, destination });
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

### 3. `public/index.html`

```html
<!DOCTYPE html>
<html lang="zh">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>端口转发控制台</title>
    <link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/5.15.4/css/all.min.css">
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
            margin: 0;
            padding: 0;
            background-color: #f5f5f5;
            color: #333;
        }
        .container {
            max-width: 600px;
            margin: auto;
            padding: 20px;
            background: #fff;
            border-radius: 12px;
            box-shadow: 0 4px 20px rgba(0, 0, 0, 0.1);
            margin-top: 30px;
        }
        h1, h2 {
            font-weight: 600;
            color: #1c1c1e;
        }
        input[type="text"], input[type="button"] {
            width: calc(100% - 22px);
            padding: 15px;
            margin: 8px 0;
            border: 1px solid #ccc;
            border-radius: 10px;
            box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
            transition: border-color 0.3s;
        }
        input[type="button"] {
            background-color: #007aff;
            color: white;
            border: none;
            cursor: pointer;
            transition: background-color 0.3s;
        }
        input[type="button"]:hover {
            background-color: #0051a8;
        }
        table {
            width: 100%;
            border-collapse: collapse;
            margin-top: 20px;
        }
        th, td {
            padding: 10px;
            border: 1px solid #ccc;
            text-align: left;
        }
        th {
            background-color: #f2f2f2;
        }
        .btn-delete {
            background-color: red;
            color: white;
            border: none;
            border-radius: 5px;
            padding: 5px 10px;
            cursor: pointer;
        }
        .btn-save {
            background-color: green;
            color: white;
            border: none;
            border-radius: 5px;
            padding: 5px 10px;
            cursor: pointer;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>端口转发控制台</h1>

        <h2>添加新规则</h2>
        <input type="text" id="startPort" placeholder="起始端口" required>
        <input type="text" id="endPort" placeholder="结束端口 (可选)">
        <input type="text" id="destination" placeholder="目标域名或localhost" required>
        <input type="button" value="添加规则" onclick="addRule()">

        <h2>当前规则</h2>
        <table id="rulesTable">
            <thead>
                <tr>
                    <th>规则类型</th>
                    <th>起始端口</th>
                    <th>结束端口</th>
                    <th>目标</th>
                    <th>操作</th>
                </tr>
            </thead>
            <tbody>
                <!-- 规则将被动态插入这里 -->
            </tbody>
        </table>
        
        <input type="button" class="btn-save" value="保存规则" onclick="saveRules()">
    </div>

    <script>
        const rules = [];
        let currentEditingIndex = -1; // 当前编辑规则的索引

        // 页面加载时获取规则
        window.onload = async () => {
            const response = await fetch('/api/rules');
            const fetchedRules = await response.json();
            fetchedRules.forEach(rule => rules.push(rule));
            renderRules();
        };

        function addRule() {
            const startPort = document.getElementById('startPort').value;
            const endPort = document.getElementById('endPort').value || '-';
            const destination = document.getElementById('destination').value;

            let type = 'SINGLE'; // 默认规则类型为单一
            if (endPort !== '-') {
                type = 'RANGE'; // 如果填写了结束端口，则为范围
            }

            if (startPort && destination) {
                if (currentEditingIndex >= 0) {
                    rules[currentEditingIndex] = { type, startPort, endPort, destination };
                    currentEditingIndex = -1; // 重置编辑状态
                } else {
                    rules.push({ type, startPort, endPort, destination });
                }
                renderRules();
                clearInputs();
            } else {
                alert('请填写所有必需的字段！');
            }
        }

        function renderRules() {
            const tableBody = document.querySelector('#rulesTable tbody');
            tableBody.innerHTML = ''; // 清空表格
            rules.forEach((rule, index) => {
                const newRow = tableBody.insertRow();
                newRow.insertCell(0).innerText = rule.type;
                newRow.insertCell(1).innerText = rule.startPort;
                newRow.insertCell(2).innerText = rule.endPort;
                newRow.insertCell(3).innerText = rule.destination;

                const editCell = newRow.insertCell(4);
                const editButton = document.createElement('button');
                editButton.innerText = '编辑';
                editButton.onclick = () => editRule(index);
                const deleteButton = document.createElement('button');
                deleteButton.innerText = '删除';
                deleteButton.className = 'btn-delete';
                deleteButton.onclick = () => deleteRule(index);
                editCell.appendChild(editButton);
                editCell.appendChild(deleteButton);
            });
        }

        function deleteRule(index) {
            rules.splice(index, 1);
            renderRules();
        }

        function editRule(index) {
            const rule = rules[index];
            document.getElementById('startPort').value = rule.startPort;
            document.getElementById('endPort').value = rule.endPort;
            document.getElementById('destination').value = rule.destination;

            currentEditingIndex = index; // 设置当前编辑的索引
        }

        function saveRules() {
            if (rules.length > 0) {
                const rulesStr = rules.map(rule =>
                    `${rule.type},${rule.startPort},${rule.endPort},${rule.destination}`).join('\n');

                fetch('/save-rules', {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json',
                    },
                    body: JSON.stringify({ rules: rulesStr })
                })
                .then(response => response.json())
                .then(data => alert(data.message))
                .catch(error => alert('保存规则时发生错误！'));
            } else {
                alert('没有规则可保存！');
            }
        }

        function clearInputs() {
            document.getElementById('startPort').value = '';
            document.getElementById('endPort').value = '';
            document.getElementById('destination').value = '';
            currentEditingIndex = -1; // 重置编辑状态
        }
    </script>
</body>
</html>
```

### 4. `public/login.html`

这是用于用户登录的页面。

```html
<!DOCTYPE html>
<html lang="zh">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>登录</title>
    <style>
        body {
            font-family: Arial, sans-serif;
            background-color: #f5f5f5;
            display: flex;
            justify-content: center;
            align-items: center;
            height: 100vh;
            margin: 0;
        }
        .login-container {
            background: #fff;
            border-radius: 8px;
            padding: 20px;
            box-shadow: 0 2px 10px rgba(0, 0, 0, 0.1);
            width: 300px;
        }
        h2 {
            margin-bottom: 20px;
            text-align: center;
        }
        input {
            width: 100%;
            padding: 10px;
            margin-bottom: 10px;
            border: 1px solid #ccc;
            border-radius: 5px;
        }
        button {
            width: 100%;
            padding: 10px;
            background-color: #007aff;
            color: #fff;
            border: none;
            border-radius: 5px;
            cursor: pointer;
        }
        button:hover {
            background-color: #0051a8;
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
    </div>

    <script>
        async function login(event) {
            event.preventDefault();
            const username = document.getElementById('username').value;
            const password = document.getElementById('password').value;

            const response = await fetch('/login', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({ username, password }),
            });

            if (response.ok) {
                window.location.href = '/'; // 成功登录后重定向到主页
            } else {
                alert('用户名或密码错误！');
            }
        }
    </script>
</body>
</html>
```

### 注意事项

1. **证书路径**：请在 `server.js` 中替换 `path/to/your/private-key.pem` 和 `path/to/your/certificate.pem` 为您的 SSL 证书和私钥的实际路径。
2. **读取权限**：确保 Node.js 进程对 `/etc/nat.conf` 和 `passwd.md` 的读取权限。
3. **密码文件**：确保 `passwd.md` 文件的格式为 `用户名:哈希密码`，例如 `admin:$2b$10$gY9KnYXxJ.PqybUkf0z2y.VD2LZX1X5LfKoJu9zW0PzW.q34654eO`。
4. **保存规则功能**：可以调整 `/save-rules` 的实现逻辑来符合实际需求。

功能包括管理用户登录、端口规则的管理等.
