以下是完整的代码，包含加密和解密密码的工具，以及原先的服务器代码和前端页面代码。确保你在本地环境中执行，记得按照注释和说明配置。

### 项目结构

```
my-port-forwarding-app/
├── bcryptTool.js
├── passwd.md
├── server.js
└── public/
    ├── index.html
    └── login.html
```

### 1. 创建 `passwd.md`

在项目根目录下创建一个名为 `passwd.md` 的文件，并保持为空，或者可以直接通过 Bcrypt 工具添加内容。

### 2. 创建 `bcryptTool.js`

在项目根目录下创建文件 `bcryptTool.js`，内容如下：

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

### 3. 创建 `server.js`

这是项目的主要服务器文件，内容如下：

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
    // 替换为您的私钥路径
    key: fs.readFileSync('path/to/your/private-key.pem'), 
    // 替换为您的证书路径
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

// 验证用户是否已登录
function isAuthenticated(req, res, next) {
    if (req.cookies.auth) {
        return next();
    } else {
        res.redirect('/login'); // 未登录则重定向到登录页
    }
}

// 处理保存规则的请求
app.post('/save-rules', isAuthenticated, (req, res) => {
    const { rules } = req.body;

    if (rules) {
        fs.writeFile('/etc/nat.conf', rules, (err) => {
            if (err) {
                console.error(err);
                return res.status(500).json({ message: '保存规则失败' });
            }
            res.json({ message: '规则已保存成功' });
        });
    } else {
        res.status(400).json({ message: '无效的规则数据' });
    }
});

// 处理登录页面的GET请求
app.get('/login', (req, res) => {
    res.sendFile(path.join(__dirname, 'public/login.html'));
});

// 错误处理
app.use((err, req, res, next) => {
    console.error(err.stack);
    res.status(500).send('服务器内发生错误！');
});

// 启动服务器
https.createServer(options, app).listen(PORT, () => {
    console.log(`服务器在 https://localhost:${PORT} 上运行`);
});
```

### 4. 创建前端页面 `public/index.html`

将以下HTML代码保存为 `public/index.html`：

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
        input[type="text"]:focus {
            border-color: #007aff;
            outline: none;
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

        @media (max-width: 768px) {
            .container {
                padding: 15px;
            }
            input[type="text"], input[type="button"] {
                padding: 12px;
            }
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>端口转发控制台</h1>

        <h2>添加新规则</h2>
        <input type="text" id="ruleType" placeholder="规则类型 (SINGLE/RANGE)">
        <input type="text" id="startPort" placeholder="起始端口">
        <input type="text" id="endPort" placeholder="结束端口 (可选)">
        <input type="text" id="destination" placeholder="目标域名或localhost">
        <input type="text" id="protocol" placeholder="协议 (tcp/udp，可选)">
        <input type="button" value="预览规则" onclick="previewRule()">
        <input type="button" value="添加规则" onclick="addRule()">

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
        
        <input type="button" class="btn-save" value="保存规则" onclick="saveRules()">
    </div>

    <script>
        const rules = [];

        function previewRule() {
            const type = document.getElementById('ruleType').value;
            const startPort = document.getElementById('startPort').value;
            const endPort = document.getElementById('endPort').value || '-';
            const destination = document.getElementById('destination').value;
            const protocol = document.getElementById('protocol').value || '-';

            if (type && startPort && destination) {
                alert(`预览规则:\n${type}, ${startPort}, ${endPort}, ${destination}, ${protocol}`);
            } else {
                alert('请填写所有必需的字段！');
            }
        }

        function addRule() {
            const type = document.getElementById('ruleType').value;
            const startPort = document.getElementById('startPort').value;
            const endPort = document.getElementById('endPort').value || '-';
            const destination = document.getElementById('destination').value;
            const protocol = document.getElementById('protocol').value || '-';

            if (type && startPort && destination) {
                rules.push({ type, startPort, endPort, destination, protocol });
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
                newRow.insertCell(4).innerText = rule.protocol;

                const deleteCell = newRow.insertCell(5);
                const deleteButton = document.createElement('button');
                deleteButton.innerText = '删除';
                deleteButton.className = 'btn-delete';
                deleteButton.onclick = () => deleteRule(index);
                deleteCell.appendChild(deleteButton);
            });
        }

        function deleteRule(index) {
            rules.splice(index, 1);
            renderRules();
        }

        function saveRules() {
            if (rules.length > 0) {
                const rulesStr = rules.map(rule =>
                    `${rule.type},${rule.startPort},${rule.endPort},${rule.destination},${rule.protocol}`).join('\n');

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
            document.getElementById('ruleType').value = '';
            document.getElementById('startPort').value = '';
            document.getElementById('endPort').value = '';
            document.getElementById('destination').value = '';
            document.getElementById('protocol').value = '';
        }
    </script>
</body>
</html>
```

### 5. 创建前端页面 `public/login.html`

将以下HTML代码保存为 `public/login.html`：

```html
<!DOCTYPE html>
<html lang="zh">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>登录</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
            display: flex;
            justify-content: center;
            align-items: center;
            height: 100vh;
            background-color: #f5f5f5;
        }
        .login-container {
            background: white;
            padding: 20px;
            border-radius: 12px;
            box-shadow: 0 0 10px rgba(0, 0, 0, 0.1);
            width: 300px;
        }
        input[type="text"], input[type="password"] {
            width: 100%;
            padding: 10px;
            margin: 10px 0;
            border: 1px solid #ccc;
            border-radius: 5px;
        }
        button {
            background-color: #007aff;
            color: white;
            border: none;
            width: 100%;
            padding: 10px;
            border-radius: 5px;
        }
    </style>
</head>
<body>

    <div class="login-container">
        <h2>登录</h2>
        <form action="/login" method="POST">
            <input type="text" name="username" required placeholder="用户名">
            <input type="password" name="password" required placeholder="密码">
            <button type="submit">登录</button>
        </form>
    </div>

</body>
</html>
```

### 启动项目

1. 确保您已安装 Node.js。
2. 在项目根目录下，使用 npm 安装所需的依赖：
   ```bash
   npm init -y
   npm install express body-parser cookie-parser bcrypt https
   ```

3. 将上述代码和文件结构创建在你的本地项目中。
4. 使用有效的 SSL 证书和私钥路径更新 `server.js` 中的对应路径。
5. 启动服务器：
   ```bash
   node server.js
   ```

6. 打开浏览器，输入 `https://localhost:3000`，您将被重定向到登录页面。

7. 使用您的默认凭据（在 `passwd.md` 文件中设置的用户名和密码）进行登录。

### 注意事项

- 确保实际操作中更新 SSL 证书路径。
- 保护 `passwd.md` 文件的访问权限以确保其安全性。 
- 每次密码更改均通过 `bcryptTool.js` 进行。 
- 定期更新依赖组件和 Node.js 以确保安全性和性能。
