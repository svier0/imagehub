use imagehub::ImageHub;
use std::path::Path;

fn config_path() -> std::path::PathBuf {
    #[cfg(target_os = "windows")]
    {
        let exe = std::env::current_exe().unwrap_or_else(|_| Path::new(".").to_path_buf());
        let dir = exe.parent().unwrap_or_else(|| Path::new("."));
        dir.join("config.ini")
    }
    #[cfg(not(target_os = "windows"))]
    {
        let base = dirs::config_dir().unwrap_or_else(|| Path::new(".").to_path_buf());
        let dir = base.join("imagehub");
        let _ = std::fs::create_dir_all(&dir);
        dir.join("config.ini")
    }
}

fn load_config() -> (String, String, String, String) {
    let path = config_path();
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return (String::new(), String::new(), String::new(), String::new()),
    };
    let mut username = String::new();
    let mut password = String::new();
    let mut cookie = String::new();
    let mut auth_token = String::new();
    for line in content.lines() {
        if let Some(v) = line.strip_prefix("username=") {
            username = v.to_string();
        } else if let Some(v) = line.strip_prefix("password=") {
            password = v.to_string();
        } else if let Some(v) = line.strip_prefix("cookie=") {
            cookie = v.to_string();
        } else if let Some(v) = line.strip_prefix("authtoken=") {
            auth_token = v.to_string();
        }
    }
    (username, password, cookie, auth_token)
}

fn save_config(
    username: &str,
    password: &str,
    cookie: &str,
    auth_token: &str,
) -> Result<(), std::io::Error> {
    let path = config_path();
    let content = format!("username={}\npassword={}\ncookie={}\nauthtoken={}\n", username, password, cookie, auth_token);
    std::fs::write(&path, content)
}

fn remove_config() -> Result<(), std::io::Error> {
    let path = config_path();
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

fn main() {
    let mut hub = ImageHub::new();
    let (cfg_user, cfg_pass, cfg_cookie, cfg_auth_token) = load_config();
    hub.set_auth(cfg_cookie, cfg_auth_token, cfg_user, cfg_pass);

    let args: Vec<String> = std::env::args().collect();

    if args.len() >= 2 && args[1] == "-i" {
        repl(&mut hub);
        return;
    }

    if args.len() < 2 {
        print_help();
        return;
    }

    match args[1].as_str() {
        "login" => cmd_login(&mut hub, &args),
        "logout" => cmd_logout(&mut hub),
        "list" => cmd_list(&mut hub),
        "upload" => cmd_upload(&mut hub, &args),
        "delete" => cmd_delete(&mut hub, &args),
        _ => print_help(),
    }
}

fn repl(hub: &mut ImageHub) {
    println!("imagehub REPL 模式，输入 /help 查看命令，/exit 退出");
    let stdin = std::io::stdin();
    loop {
        let mut line = String::new();
        print!("imagehub> ");
        use std::io::Write;
        std::io::stdout().flush().ok();
        if stdin.read_line(&mut line).is_err() || line.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.trim().split_whitespace().collect();
        let cmd = parts[0].strip_prefix('/').unwrap_or(parts[0]);
        match cmd {
            "exit" | "quit" => {
                println!("再见");
                break;
            }
            "help" => print_repl_help(),
            "login" => {
                if parts.len() < 3 {
                    eprintln!("用法: /login <用户名> <密码>");
                } else {
                    match hub.login(parts[1], parts[2]) {
                        Ok(()) => println!("登录成功"),
                        Err(e) => eprintln!("登录失败: {}", e),
                    }
                }
            }
            "list" => cmd_list(hub),
            "upload" => {
                if parts.len() < 2 {
                    eprintln!("用法: /upload <文件路径>");
                } else {
                    match hub.upload_image(parts[1]) {
                        Ok(info) => println!("上传成功: [{}] {} {}", info.id, info.title, info.url),
                        Err(e) => eprintln!("上传失败: {}", e),
                    }
                }
            }
            "delete" => {
                if parts.len() < 2 {
                    eprintln!("用法: /delete <图片ID>");
                } else {
                    match hub.delete_image(parts[1]) {
                        Ok(()) => println!("删除成功"),
                        Err(e) => eprintln!("删除失败: {}", e),
                    }
                }
            }
            _ => println!("未知命令: {}，输入 /help 查看可用命令", cmd),
        }
    }
}

fn print_repl_help() {
    println!("可用命令:");
    println!("  /login <用户名> <密码>    登录");
    println!("  /list                   查看图片列表");
    println!("  /upload <文件路径>      上传图片");
    println!("  /delete <图片ID>        删除图片");
    println!("  /exit                   退出");
    println!("  /help                   显示本帮助");
}

fn print_help() {
    let prog = std::env::args().next().unwrap_or_else(|| "imagehub".into());
    eprintln!("用法: {} [-i | <命令> [参数]]", prog);
    eprintln!("选项:");
    eprintln!("  -i                      进入交互式 REPL 模式");
    eprintln!("命令:");
    eprintln!("  login <用户名> <密码>   登录（持久化账号密码和 cookie）");
    eprintln!("  logout                  登出（清除持久化的账号密码和 cookie）");
    eprintln!("  list                    查看图片列表");
    eprintln!("  upload <文件路径>       上传图片");
    eprintln!("  delete <图片ID>         删除图片");
}

fn cmd_login(hub: &mut ImageHub, args: &[String]) {
    if args.len() < 4 {
        eprintln!("用法: {} login <用户名> <密码>", args[0]);
        return;
    }
    let username = &args[2];
    let password = &args[3];

    match hub.login(username, password) {
        Ok(()) => {
            let (cookie, auth_token, _, _) = hub.get_auth();
            if let Err(e) = save_config(username, password, cookie, auth_token) {
                eprintln!("保存配置失败: {}", e);
                return;
            }
            println!("登录成功");
        }
        Err(e) => eprintln!("登录失败: {}", e),
    }
}

fn cmd_logout(hub: &mut ImageHub) {
    hub.set_auth(String::new(), String::new(), String::new(), String::new());
    if let Err(e) = remove_config() {
        eprintln!("清除配置失败: {}", e);
    } else {
        println!("已登出");
    }
}

fn cmd_list(hub: &mut ImageHub) {
    match hub.list_images() {
        Ok(images) => {
            if images.is_empty() {
                println!("没有图片");
            } else {
                for img in &images {
                    println!("[{}] {} - {}", img.id, img.title, img.url);
                }
            }
        }
        Err(e) => eprintln!("获取图片列表失败: {}", e),
    }
}

fn cmd_upload(hub: &mut ImageHub, args: &[String]) {
    if args.len() < 3 {
        eprintln!("用法: {} upload <文件路径>", args[0]);
        return;
    }
    match hub.upload_image(&args[2]) {
        Ok(info) => println!("上传成功: [{}] {} {}", info.id, info.title, info.url),
        Err(e) => eprintln!("上传失败: {}", e),
    }
}

fn cmd_delete(hub: &mut ImageHub, args: &[String]) {
    if args.len() < 3 {
        eprintln!("用法: {} delete <图片ID>", args[0]);
        return;
    }
    match hub.delete_image(&args[2]) {
        Ok(()) => println!("删除成功"),
        Err(e) => eprintln!("删除失败: {}", e),
    }
}
