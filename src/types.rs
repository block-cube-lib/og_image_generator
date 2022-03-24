#[derive(Debug, Clone)]
pub struct OgpInfo {
    pub title: String,
    pub thumbnail_url: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

impl Size {
    pub fn resize_to_fit(&mut self, size: Size) {
        match (size.width < self.width, size.height < self.height) {
            (true, false) => {
                let rate = size.width as f64 / self.width as f64;
                self.width = (self.width as f64 * rate) as u32;
                self.height = (self.height as f64 * rate) as u32;
            },
            (false, true) => {
                let rate = size.height as f64 / self.height as f64;
                self.width = (self.width as f64 * rate) as u32;
                self.height = (self.height as f64 * rate) as u32;
            },
            (true, true) => {
                let rate_w = size.width as f64 / self.width as f64;
                let rate_h = size.height as f64 / self.height as f64;
                let rate = rate_w.min(rate_h);
                self.width = (self.width as f64 * rate) as u32;
                self.height = (self.height as f64 * rate) as u32;
            },
            _ => {},
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Ancher {
    LeftTop,
    LeftBottom,
    RightBottom,
}

#[derive(Debug, Clone, Copy)]
pub struct Offset {
    pub ancher: Ancher,
    pub x: i64,
    pub y: i64,
}
