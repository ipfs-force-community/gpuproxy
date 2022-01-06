#[derive(Clone, Debug)]
pub struct ServiceConfig  {
    pub url: String
}


impl ServiceConfig {
    pub fn new(url: String) -> Self {
        Self {
            url
        }
    }
}
