pub trait Optionable {
  fn to_option(self) -> Option<String>;
}

impl Optionable for String {
  fn to_option(self) -> Option<String> {
    if self.is_empty() {
      None
    } else {
      Some(self)
    }
  }
}
