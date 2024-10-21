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
    key: fs.readFileSync('/path/to/your/private-key.pem'),
    cert: fs.readFileSync('/path/to/your/certificate.pem')
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
                destination: parts[3]
            };
        });
    });
};
readRulesFile();

function isAuthenticated(req, res, next) {
    if (req.cookies.auth) {
        return next();
    } else {
        res.redirect('/login'); // 未登录则重定向到登录页
    }
}

app.get('/', isAuthenticated, (req, res) => {
    res.sendFile(path.join(__dirname, 'public/index.html'));
});

app.post('/login', async (req, res) => {
    const { username, password } = req.body;
    const hashedPassword = users[username];

    if (hashedPassword && await bcrypt.compare(password, hashedPassword)) {
        res.cookie('auth', '1');
        res.redirect('/');
    } else {
        res.status(401).send('用户名或密码错误');
    }
});

app.get('/login', (req, res) => {
    if (req.cookies.auth) {
        return res.redirect('/');
    }
    res.sendFile(path.join(__dirname, 'public/login.html'));
});

// 获取规则
app.get('/api/rules', (req, res) => {
    res.json(rules);
});

// 编辑规则
app.post('/edit-rule', (req, res) => {
    const { index, startPort, endPort, destination } = req.body;
    if (index < 0 || index >= rules.length) {
        return res.status(400).json({ message: '无效的规则索引' });
    }
    
    rules[index] = {
        ...rules[index],
        startPort,
        endPort,
        destination
    };
    res.json({ message: '规则编辑成功' });
});

// 处理保存规则的请求
app.post('/save-rules', (req, res) => {
    const rulesData = req.body.rules.map(rule => {
        const endPort = rule.endPort ? rule.endPort : rule.startPort;
        return `${rule.type},${rule.startPort},${endPort},${rule.destination}`;
    }).join('\n');

    fs.writeFile('/etc/nat.conf', rulesData, (err) => {
        if (err) {
            return res.status(500).json({ message: '保存规则失败' });
        }
        readRulesFile(); // 重新加载规则
        res.json({ message: '规则保存成功' });
    });
});

// 删除规则
app.post('/delete-rule', (req, res) => {
    const index = req.body.index;
    if (index < 0 || index >= rules.length) {
        return res.status(400).json({ message: '无效的规则索引' });
    }
    
    rules.splice(index, 1);
    res.json({ message: '规则删除成功' });
});

// 处理预览规则的请求
app.get('/api/rules/preview', (req, res) => {
    const previewRules = rules.map(rule => `${rule.type},${rule.startPort},${rule.endPort || rule.startPort},${rule.destination}`);
    res.json(previewRules);
});

// 登出
app.post('/logout', (req, res) => {
    res.clearCookie('auth');
    res.redirect('/login');
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
