use imagehub::services::ImageHub;
use std::path::Path;

fn config_path() -> std::path::PathBuf {
    let exe = std::env::current_exe().unwrap_or_else(|_| Path::new(".").to_path_buf());
    let dir = exe.parent().unwrap_or_else(|| Path::new("."));
    dir.join("config.ini")
}

fn load_config() -> (Option<String>, Option<String>, Option<String>, Option<String>) {
    let path = config_path();
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return (None, None, None, None),
    };
    let mut username = None;
    let mut password = None;
    let mut cookie = None;
    let mut auth_token = None;
    for line in content.lines() {
        if let Some(v) = line.strip_prefix("username=") {
            username = Some(v.to_string());
        } else if let Some(v) = line.strip_prefix("password=") {
            password = Some(v.to_string());
        } else if let Some(v) = line.strip_prefix("cookie=") {
            cookie = Some(v.to_string());
        } else if let Some(v) = line.strip_prefix("authtoken=") {
            auth_token = Some(v.to_string());
        }
    }
    (username, password, cookie, auth_token)
}

fn save_config(
    username: &str,
    password: &str,
    cookie: Option<&str>,
    auth_token: Option<&str>,
) -> Result<(), std::io::Error> {
    let path = config_path();
    let mut content = format!("username={}\npassword={}\n", username, password);
    if let Some(c) = cookie {
        content.push_str(&format!("cookie={}\n", c));
    }
    if let Some(t) = auth_token {
        content.push_str(&format!("authtoken={}\n", t));
    }
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

    unsafe {
        if let Some(u) = &cfg_user {
            imagehub::config::USERNAME = Box::leak(u.clone().into_boxed_str());
        }
        if let Some(p) = &cfg_pass {
            imagehub::config::PASSWORD = Box::leak(p.clone().into_boxed_str());
        }
    }
    if let (Some(c), Some(t)) = (cfg_cookie, cfg_auth_token) {
        hub.set_session(c, t);
    }

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
        let mut fake_args = vec!["imagehub".to_string(), cmd.to_string()];
        for p in &parts[1..] {
            fake_args.push(p.to_string());
        }
        match cmd {
            "exit" | "quit" => {
                println!("再见");
                break;
            }
            "help" => print_repl_help(),
            "login" => cmd_login_repl(hub, &fake_args),
            "config" => cmd_config(hub, &fake_args),
            "list" => cmd_list(hub),
            "upload" => cmd_upload(hub, &fake_args),
            "delete" => cmd_delete(hub, &fake_args),
            _ => println!("未知命令: {}，输入 /help 查看可用命令", cmd),
        }
    }
}

fn print_repl_help() {
    println!("可用命令:");
    println!("  /login [<用户名> <密码>]  验证登录（可选指定账号，否则用已配置的）");
    println!("  /config <用户名> <密码>  设置账号密码");
    println!("  /list                   查看图片列表");
    println!("  /upload <文件路径>      上传图片");
    println!("  /delete <图片ID>        删除图片");
    println!("  /exit                   退出");
    println!("  /help                   显示本帮助");
}

fn print_help() {
    eprintln!("用法: imagehub [-i | <命令> [参数]]");
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
        eprintln!("用法: cli login <用户名> <密码>");
        return;
    }
    let username = &args[2];
    let password = &args[3];

    unsafe {
        imagehub::config::USERNAME = Box::leak(username.clone().into_boxed_str());
        imagehub::config::PASSWORD = Box::leak(password.clone().into_boxed_str());
    }

    hub.clear_session();
    match imagehub::api::login(username, password) {
        Ok((cookie, auth_token)) => {
            hub.set_session(cookie.clone(), auth_token.clone());
            if let Err(e) = save_config(username, password, Some(&cookie), Some(&auth_token)) {
                eprintln!("保存配置失败: {}", e);
            } else {
                println!("登录成功");
            }
        }
        Err(e) => eprintln!("登录失败: {}", e),
    }
}

fn cmd_logout(hub: &mut ImageHub) {
    hub.clear_session();
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
        eprintln!("用法: cli upload <文件路径>");
        return;
    }
    match hub.upload_image(&args[2]) {
        Ok(info) => println!("上传成功: [{}] {} {}", info.id, info.title, info.url),
        Err(e) => eprintln!("上传失败: {}", e),
    }
}

fn cmd_config(hub: &mut ImageHub, args: &[String]) {
    if args.len() < 4 {
        eprintln!("用法: /config <用户名> <密码>");
        return;
    }
    let username = &args[2];
    let password = &args[3];
    unsafe {
        imagehub::config::USERNAME = Box::leak(username.clone().into_boxed_str());
        imagehub::config::PASSWORD = Box::leak(password.clone().into_boxed_str());
    }
    hub.clear_session();
    println!("账号密码已设置");
}

fn cmd_login_repl(hub: &mut ImageHub, args: &[String]) {
    let (username, password): (&str, &str) = if args.len() >= 4 {
        (args[2].as_str(), args[3].as_str())
    } else {
        (unsafe { imagehub::config::USERNAME }, unsafe { imagehub::config::PASSWORD })
    };
    match imagehub::api::login(username, password) {
        Ok((cookie, auth_token)) => {
            if args.len() >= 4 {
                unsafe {
                    imagehub::config::USERNAME = Box::leak(username.to_string().into_boxed_str());
                    imagehub::config::PASSWORD = Box::leak(password.to_string().into_boxed_str());
                }
            }
            hub.set_session(cookie, auth_token);
            println!("登录成功");
        }
        Err(e) => eprintln!("登录失败: {}", e),
    }
}

fn cmd_delete(hub: &mut ImageHub, args: &[String]) {
    if args.len() < 3 {
        eprintln!("用法: cli delete <图片ID>");
        return;
    }
    match hub.delete_image(&args[2]) {
        Ok(()) => println!("删除成功"),
        Err(e) => eprintln!("删除失败: {}", e),
    }
}
