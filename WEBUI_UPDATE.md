# WebUI 更新说明

## 更新内容

### 1. 创建共享配置库 (nat-common)

创建了一个新的共享库 `nat-common`，包含：

- **TomlConfig 结构体**：配置的核心数据结构
- **Rule 枚举**：支持三种规则类型
  - `Single`: 单端口转发
  - `Range`: 端口范围转发
  - `Redirect`: 端口重定向
- **验证功能**：
  - `TomlConfig::validate()`: 验证整个配置
  - `Rule::validate()`: 验证单个规则
  - 验证协议（tcp/udp/all）
  - 验证 IP 版本（ipv4/ipv6/both）
  - 验证端口范围（1-65535）
  - 验证必填字段（domain）

### 2. WebUI 后端更新

#### config.rs

- 移除本地 TomlConfig 和 Rule 定义
- 使用 nat-common 的共享结构体
- ConfigFormat::Toml 现在直接存储 TOML 字符串而不是解析后的对象
- 添加 `validate()` 方法用于配置验证

#### handlers.rs

- **get_config()**：
  - 返回格式：`{ format: "toml", content: "..." }`
  - content 现在是原始 TOML 字符串，而不是 JSON 对象
- **save_config()**：
  - 接收原始 TOML 字符串
  - 使用 `TomlConfig::from_toml_str()` 进行验证
  - 验证失败时返回详细的错误信息
  - 验证成功后再保存到文件

### 3. WebUI 前端更新

#### index.html

- **loadConfig()**：
  - 直接显示 TOML 字符串到 textarea
  - 不再使用 JSON.stringify() 转换
- **saveConfig()**：
  - 直接发送 textarea 中的内容（TOML 字符串）
  - 不再解析为 JSON 对象

### 4. NAT CLI 更新

#### nat-cli/Cargo.toml

- 添加 nat-common 依赖

#### nat-cli/src/config.rs

- 使用 nat-common 的 TomlConfig 和 Rule
- 移除本地重复定义
- `read_toml_config()` 使用 nat-common 的解析和验证功能
- `toml_example()` 使用 nat-common 的 to_toml_string() 方法

## 使用示例

### TOML 配置格式

```toml
[[rules]]
type = "single"
sport = 10000
dport = 443
domain = "baidu.com"
protocol = "all"
ip_version = "ipv4"
comment = "百度HTTPS服务转发"

[[rules]]
type = "range"
port_start = 20000
port_end = 20100
domain = "google.com"
protocol = "tcp"
ip_version = "both"
comment = "端口范围转发"

[[rules]]
type = "redirect"
sport = 8080
sport_end = 8090  # 可选，用于端口范围重定向
dport = 3128
protocol = "all"
ip_version = "ipv4"
comment = "端口重定向"
```

### WebUI 界面展示

现在在 WebUI 中：

1. **配置编辑**标签页会直接显示 TOML 格式的配置
2. 用户可以直接编辑 TOML 文本
3. 保存时会自动验证配置是否合法
4. 验证失败会显示详细的错误信息

### 配置验证

配置会在以下情况下自动验证：

1. **WebUI 保存时**：

   - 协议必须是 tcp、udp 或 all
   - IP 版本必须是 ipv4、ipv6 或 both
   - 端口必须在 1-65535 范围内
   - Single 和 Range 规则必须包含 domain 字段

2. **NAT CLI 启动时**：
   - 读取 TOML 配置时自动验证
   - 验证失败会阻止程序启动

## 优势

1. **代码复用**：配置结构和验证逻辑在 nat-cli 和 webui 之间共享
2. **类型安全**：使用 Rust 的类型系统保证配置的正确性
3. **用户友好**：WebUI 直接展示和编辑 TOML 格式，更符合直觉
4. **错误提示**：验证失败时提供详细的错误信息
5. **维护性**：配置结构的变更只需在 nat-common 中修改一次

## 测试

可以使用 test_nat.toml 测试配置验证功能：

```bash
# 启动 WebUI（使用 HTTPS）
./target/release/webui \
  --username admin \
  --password admin123 \
  --port 8444 \
  --cert /path/to/cert.pem \
  --key /path/to/key.pem \
  --toml-config /root/nftables-nat-rust/test_nat.toml

# 启动 WebUI（使用 HTTP）
./target/release/webui \
  --username admin \
  --password admin123 \
  --port 9999 \
  --toml-config /root/nftables-nat-rust/test_nat.toml
```

访问 WebUI 后，在"配置编辑"标签页可以看到 TOML 格式的配置，可以直接编辑并保存。
