use regex::Regex;
use reqwest::header;

use crate::models::ImageInfo;
use crate::{Error, Result};

const BASE_URL: &str = "https://www.imagehub.cc";

fn extract_set_cookie_value(resp: &reqwest::blocking::Response, name: &str) -> Option<String> {
    for value in resp.headers().get_all(header::SET_COOKIE) {
        let s = value.to_str().ok()?;
        if s.starts_with(&format!("{}=", name)) {
            return s.split(';').next().map(|v| v.to_string());
        }
    }
    None
}

fn build_client() -> Result<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| Error::HttpError(e.to_string()))
}

pub fn login(username: &str, password: &str) -> Result<(String, String)> {
    let client = build_client()?;
    let login_url = format!("{}/login", BASE_URL);

    let resp = client
        .get(&login_url)
        .header(header::USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .send()
        .map_err(|e| Error::HttpError(format!("GET /login 失败: {}", e)))?;

    let phpsessid = extract_set_cookie_value(&resp, "PHPSESSID")
        .ok_or_else(|| Error::HttpError("未收到 PHPSESSID".to_string()))?;

    let body = resp.text().map_err(|e| Error::HttpError(e.to_string()))?;

    let re = Regex::new(r#"PF\.obj\.config\.auth_token = "([^"]+)""#)
        .map_err(|e| Error::HttpError(format!("正则编译失败: {}", e)))?;
    let auth_token = re
        .captures(&body)
        .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
        .ok_or_else(|| Error::HttpError("无法从页面提取 auth_token".to_string()))?;

    let resp = client
        .post(&login_url)
        .header(header::USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .header(header::REFERER, &login_url)
        .header(header::COOKIE, &phpsessid)
        .form(&[
            ("login-subject", username),
            ("password", password),
            ("auth_token", &auth_token),
        ])
        .send()
        .map_err(|e| Error::HttpError(format!("POST /login 失败: {}", e)))?;

    let keep_login = extract_set_cookie_value(&resp, "KEEP_LOGIN")
        .ok_or_else(|| Error::HttpError("登录失败，未收到 KEEP_LOGIN".to_string()))?;

    let cookie = format!("{}; {}", phpsessid, keep_login);
    Ok((cookie, auth_token))
}

pub fn list_images(cookie: &str, username: &str) -> Result<Vec<ImageInfo>> {
    if username.is_empty() {
        return Err(Error::HttpError("用户名未设置".to_string()));
    }

    let url = format!("{}/{}/?list=images&sort=date_desc&page=1", BASE_URL, username);

    let client = build_client()?;
    let resp = client
        .get(&url)
        .header(header::COOKIE, cookie)
        .header(header::USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .send()
        .map_err(|e| Error::HttpError(format!("GET 列表失败: {}", e)))?;

    let body = resp.text().map_err(|e| Error::HttpError(e.to_string()))?;
    parse_image_list(&body)
}

fn percent_decode(s: &str) -> Result<String> {
    let bytes = percent_encoding::percent_decode(s.as_bytes());
    Ok(bytes.decode_utf8().map_err(|e| Error::HttpError(format!("URL 解码失败: {}", e)))?.to_string())
}

fn parse_image_list(html: &str) -> Result<Vec<ImageInfo>> {
    let start = html.find(r#"class="pad-content-listing""#)
        .ok_or_else(|| Error::HttpError("未找到 pad-content-listing".to_string()))?;
    let mut html = &html[start..];

    let mut images = Vec::new();
    loop {
        let Some(obj_start) = html.find("data-object=") else {
            return Ok(images);
        };
        let after_eq = obj_start + "data-object=".len();
        if after_eq >= html.len() {
            return Ok(images);
        }
        let quote = html.as_bytes()[after_eq] as char;
        if quote != '"' && quote != '\'' {
            return Err(Error::HttpError("data-object 值的引号格式异常".to_string()));
        }
        let value_start = after_eq + 1;
        let value_end = html[value_start..].find(quote)
            .map(|pos| value_start + pos)
            .ok_or_else(|| Error::HttpError("data-object 值缺少结束引号".to_string()))?;

        let raw = &html[value_start..value_end];
        let json_str = percent_decode(raw)?;
        let obj: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| Error::HttpError(format!("JSON 解析失败: {}", e)))?;

        let img = obj.get("image").and_then(|v| v.as_object());

        images.push(ImageInfo {
            id: obj_get(&obj, "id_encoded"),
            title: obj_get(&obj, "title"),
            url: obj_get(&obj, "url"),
            mime: img.and_then(|m| m.get("mime")).and_then(|v| v.as_str()).unwrap_or("").to_string(),
            extension: img.and_then(|m| m.get("extension")).and_then(|v| v.as_str()).unwrap_or("").to_string(),
            name: img.and_then(|m| m.get("name")).and_then(|v| v.as_str()).unwrap_or("").to_string(),
            filename: img.and_then(|m| m.get("filename")).and_then(|v| v.as_str()).unwrap_or("").to_string(),
            size: img.and_then(|m| m.get("size")).and_then(|v| v.as_str()).unwrap_or("").to_string(),
            width: obj_get(&obj, "width"),
            height: obj_get(&obj, "height"),
        });

        html = &html[value_end + 1..];
    }
}

fn obj_get(obj: &serde_json::Value, key: &str) -> String {
    obj.get(key).and_then(|v| v.as_str()).unwrap_or("").to_string()
}

pub fn upload_image(cookie: &str, auth_token: &str, file_path: &str) -> Result<ImageInfo> {
    use reqwest::blocking::multipart;

    let client = build_client()?;
    let url = format!("{}/json", BASE_URL);

    let path = std::path::Path::new(file_path);
    let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("file");
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
    let mime_str = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        _ => "application/octet-stream",
    };

    let file_bytes = std::fs::read(file_path)
        .map_err(|e| Error::IoError(e))?;
    let checksum = md5::compute(&file_bytes);
    let checksum_hex = format!("{:x}", checksum);
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| Error::HttpError(format!("时间戳获取失败: {}", e)))?
        .as_millis()
        .to_string();

    let part = multipart::Part::bytes(file_bytes)
        .file_name(file_name.to_string())
        .mime_str(mime_str)
        .map_err(|e| Error::HttpError(e.to_string()))?;

    let form = multipart::Form::new()
        .part("source", part)
        .text("type", "file")
        .text("action", "upload")
        .text("timestamp", timestamp)
        .text("auth_token", auth_token.to_string())
        .text("nsfw", "0")
        .text("mimetype", mime_str.to_string())
        .text("checksum", checksum_hex);

    let resp = client
        .post(&url)
        .header(header::USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .header(header::COOKIE, cookie)
        .multipart(form)
        .send()
        .map_err(|e| Error::HttpError(format!("POST 上传失败: {}", e)))?;

    let body = resp.text().map_err(|e| Error::HttpError(e.to_string()))?;
    let json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| Error::HttpError(format!("JSON 解析失败: {}", e)))?;

    if json.get("success").is_some() {
        let img = json.get("image").ok_or_else(|| Error::HttpError("响应缺少 image 字段".to_string()))?;
        Ok(ImageInfo {
            id: obj_get(img, "id_encoded"),
            title: obj_get(img, "title"),
            url: obj_get(img, "url"),
            mime: obj_get(img, "mime"),
            extension: obj_get(img, "extension"),
            name: obj_get(img, "name"),
            filename: obj_get(img, "filename"),
            size: img.get("size").and_then(|v| v.as_i64()).map(|v| v.to_string()).unwrap_or_default(),
            width: img.get("width").and_then(|v| v.as_i64()).map(|v| v.to_string()).unwrap_or_default(),
            height: img.get("height").and_then(|v| v.as_i64()).map(|v| v.to_string()).unwrap_or_default(),
        })
    } else if let Some(err) = json.get("error") {
        let msg = err.get("message").and_then(|v| v.as_str()).unwrap_or("未知错误");
        let code = err.get("code").and_then(|v| v.as_i64()).unwrap_or(0);
        Err(Error::HttpError(format!("上传失败 [{}]: {}", code, msg)))
    } else {
        Err(Error::HttpError(format!("上传失败: {}", body)))
    }
}

pub fn delete_image(cookie: &str, auth_token: &str, image_id: &str) -> Result<()> {
    let client = build_client()?;
    let url = format!("{}/json", BASE_URL);

    let resp = client
        .post(&url)
        .header(header::USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .header(header::COOKIE, cookie)
        .header("X-Requested-With", "XMLHttpRequest")
        .form(&[
            ("auth_token", auth_token),
            ("action", "delete"),
            ("single", "true"),
            ("delete", "image"),
            ("deleting[id]", image_id),
        ])
        .send()
        .map_err(|e| Error::HttpError(format!("POST 删除失败: {}", e)))?;

    let body = resp.text().map_err(|e| Error::HttpError(e.to_string()))?;
    let json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| Error::HttpError(format!("JSON 解析失败: {}", e)))?;

    if json.get("success").is_some() {
        Ok(())
    } else if let Some(err) = json.get("error") {
        let msg = err.get("message").and_then(|v| v.as_str()).unwrap_or("未知错误");
        let code = err.get("code").and_then(|v| v.as_i64()).unwrap_or(0);
        Err(Error::HttpError(format!("删除失败 [{}]: {}", code, msg)))
    } else {
        Err(Error::HttpError(format!("删除失败: {}", body)))
    }
}
