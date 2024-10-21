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
