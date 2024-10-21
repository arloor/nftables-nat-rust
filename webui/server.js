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
    key: fs.readFileSync('path/to/your/private-key.pem'),  // 替换为
