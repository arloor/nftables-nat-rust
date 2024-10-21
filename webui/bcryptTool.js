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
