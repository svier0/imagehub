use crate::models::ImageInfo;
use crate::Result;

pub struct ImageHub {
    cookie: Option<String>,
    auth_token: Option<String>,
}

impl ImageHub {
    pub fn new() -> Self {
        ImageHub { cookie: None, auth_token: None }
    }

    pub fn set_session(&mut self, cookie: String, auth_token: String) {
        self.cookie = Some(cookie);
        self.auth_token = Some(auth_token);
    }

    pub fn take_cookie(&self) -> Option<&str> {
        self.cookie.as_deref()
    }

    pub fn take_auth_token(&self) -> Option<&str> {
        self.auth_token.as_deref()
    }

    pub fn clear_session(&mut self) {
        self.cookie = None;
        self.auth_token = None;
    }

    fn ensure_session(&mut self) -> Result<(&str, &str)> {
        if self.cookie.is_some() && self.auth_token.is_some() {
            return Ok((self.cookie.as_ref().unwrap(), self.auth_token.as_ref().unwrap()));
        }
        let (cookie, auth_token) = crate::api::login(
            unsafe { crate::config::USERNAME },
            unsafe { crate::config::PASSWORD },
        )?;
        self.cookie = Some(cookie);
        self.auth_token = Some(auth_token);
        Ok((self.cookie.as_ref().unwrap(), self.auth_token.as_ref().unwrap()))
    }

    pub fn list_images(&mut self) -> Result<Vec<ImageInfo>> {
        let (cookie, _) = self.ensure_session()?;
        crate::api::list_images(cookie)
    }

    pub fn upload_image(&mut self, file_path: &str) -> Result<ImageInfo> {
        let (cookie, auth_token) = self.ensure_session()?;
        crate::api::upload_image(cookie, auth_token, file_path)
    }

    pub fn delete_image(&mut self, image_id: &str) -> Result<()> {
        let (cookie, auth_token) = self.ensure_session()?;
        crate::api::delete_image(cookie, auth_token, image_id)
    }
}
