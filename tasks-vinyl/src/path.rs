use std::fmt;

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Path(String);

impl Path {
    pub fn new(path: impl ToString) -> Path {
        Path(path.to_string())
    }

    pub fn filename(&self) -> Option<&str> {
        pathutils::filename(&self.0)
    }

    pub fn set_filename<S: AsRef<str>>(&mut self, filename: S) -> &mut Self {
        self.0 = pathutils::set_filename(&self.0, filename.as_ref());
        self
    }

    pub fn ext(&self) -> Option<&str> {
        pathutils::extname(&self.0)
    }

    pub fn set_ext(&mut self, ext: &str) -> &mut Self {
        self.0 = pathutils::set_extname(&self.0, ext);
        self
    }

    pub fn join<S: AsRef<str>>(&self, path: S) -> Path {
        Path(pathutils::join(&self.0, path.as_ref()))
    }
}

impl std::ops::Deref for Path {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<String> for Path {
    fn from(path: String) -> Self {
        Path::new(path)
    }
}

impl<'a> From<&'a str> for Path {
    fn from(path: &'a str) -> Self {
        Path::new(path)
    }
}
