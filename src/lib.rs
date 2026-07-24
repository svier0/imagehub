//! # imagehub
//!
//! 用于管理 [imagehub.cc](https://www.imagehub.cc) 账号图片资源的 Rust 库。
//!
//! ## 用法
//!
//! ```rust
//! let mut hub = imagehub::ImageHub::new();
//! hub.login("username", "password").unwrap();
//! let _images = hub.list_images().unwrap();
//! ```

pub(crate) mod api;
/// 错误类型定义。
pub mod errors;
/// 图片信息数据结构。
pub mod models;

pub use errors::Error;
pub use models::ImageInfo;

pub type Result<T> = std::result::Result<T, Error>;

/// imagehub.cc 账号管理器。
///
/// 管理登录会话和认证凭据，提供图片的增删查操作。
/// 需要先调用 [`login`](ImageHub::login) 或在启动时通过 [`set_auth`](ImageHub::set_auth) 恢复会话。
pub struct ImageHub {
    cookie: String,
    auth_token: String,
    username: String,
    password: String,
}

impl ImageHub {
    /// 创建一个新的空实例。
    pub fn new() -> Self {
        ImageHub { cookie: String::new(), auth_token: String::new(), username: String::new(), password: String::new() }
    }

    /// 直接设置完整的认证信息（cookie、auth_token、用户名、密码）。
    ///
    /// 通常用于从本地配置文件恢复上次登录的会话。
    pub fn set_auth(&mut self, cookie: String, auth_token: String, username: String, password: String) {
        self.cookie = cookie;
        self.auth_token = auth_token;
        self.username = username;
        self.password = password;
    }

    /// 获取当前保存的认证信息，返回 `(cookie, auth_token, username, password)`。
    pub fn get_auth(&self) -> (&str, &str, &str, &str) {
        (&self.cookie, &self.auth_token, &self.username, &self.password)
    }

    /// 用用户名和密码登录，成功后自动保存 session。
    ///
    /// 调用前可用 [`set_auth`](ImageHub::set_auth) 恢复之前的会话以避免重复登录。
    pub fn login(&mut self, username: &str, password: &str) -> Result<()> {
        let (cookie, auth_token) = api::login(username, password)?;
        self.cookie = cookie;
        self.auth_token = auth_token;
        self.username = username.to_string();
        self.password = password.to_string();
        Ok(())
    }

    fn ensure_session(&mut self) -> Result<(&str, &str)> {
        if !self.cookie.is_empty() && !self.auth_token.is_empty() {
            return Ok((&self.cookie, &self.auth_token));
        }
        if !self.username.is_empty() && !self.password.is_empty() {
            let (cookie, auth_token) = api::login(&self.username, &self.password)?;
            self.cookie = cookie;
            self.auth_token = auth_token;
            return Ok((&self.cookie, &self.auth_token));
        }
        Err(Error::NotLoggedIn)
    }

    /// 获取当前账号的图片列表。
    pub fn list_images(&mut self) -> Result<Vec<ImageInfo>> {
        let cookie = self.ensure_session()?.0.to_string();
        let username = self.username.clone();
        api::list_images(&cookie, &username)
    }

    /// 上传图片到当前账号。
    ///
    /// `file_path` 为本地图片文件路径。返回上传后的图片信息。
    pub fn upload_image(&mut self, file_path: &str) -> Result<ImageInfo> {
        let (cookie, auth_token) = self.ensure_session()?;
        api::upload_image(cookie, auth_token, file_path)
    }

    /// 删除指定 ID 的图片。
    pub fn delete_image(&mut self, image_id: &str) -> Result<()> {
        let (cookie, auth_token) = self.ensure_session()?;
        api::delete_image(cookie, auth_token, image_id)
    }
}
