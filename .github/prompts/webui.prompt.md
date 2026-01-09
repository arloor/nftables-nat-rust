---
agent: agent
tools: ['vscode', 'execute', 'read', 'edit', 'search', 'web', 'agent', 'github/*', 'todo']
---


我需要给这个nat转发程序写一个webui，作用有两个：

1. 修改规则，需要支持传统配置和TOML配置。具体使用什么版本配置由systemd服务文件决定
2. 展示最终的nftables规则，self-nat和self-nat6表

在技术栈上，需要使用 axum-bootstrap 、jsonwebtoken这些库来实现WebUI的功能。需要有登录和鉴权功能。用户名和密码由命令行参数提供。由于需要用户输入用户名和密码进行登录，因此需要支持TLS。登录表单需要CSRF保护。

你可以从我的这个项目中https://github.com/arloor/guba_rank_stock_js中找到如何使用axum-bootstrap 、jsonwebtoken来实现WebUI的登录和鉴权功能。

对了，你需要将这个项目改成workspace来支持多binary