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

// 错误处理
app.use((err, req, res, next) => {
    console.error(err.stack);
    res.status(500).send('服务器内发生错误！');
});

// 启动服务器
https.createServer(options, app).listen(PORT, () => {
    console.log(`服务器在 https://localhost:${PORT} 上运行`);
});
