# ImageHub CLI

命令行工具，用于管理 [imagehub.cc](https://www.imagehub.cc) 账号下的图片资源。

## 功能

- 登录 / 登出（持久化会话）
- 查看图片列表
- 上传图片
- 删除图片

## 下载

从 [Releases](https://github.com/viernqmo/imagehub/releases) 下载对应平台的可执行文件。

## 使用

### 单次命令模式

```bash
# 登录（持久化账号密码和会话）
imagehub login <用户名> <密码>

# 查看图片列表
imagehub list

# 上传图片
imagehub upload <文件路径>

# 删除图片
imagehub delete <图片ID>

# 登出（清除持久化的账号密码和会话）
imagehub logout
```

### 交互式 REPL 模式

```bash
imagehub -i
```

进入交互式 shell，支持以下命令：

```
imagehub> /config <用户名> <密码>  设置账号密码
imagehub> /login [<用户名> <密码>]  验证登录
imagehub> /list                    查看图片列表
imagehub> /upload <文件路径>       上传图片
imagehub> /delete <图片ID>         删除图片
imagehub> /exit                    退出
imagehub> /help                    帮助
```

## 配置文件

配置文件 `config.ini` 存储在可执行文件所在目录，包含登录凭据和会话信息。
